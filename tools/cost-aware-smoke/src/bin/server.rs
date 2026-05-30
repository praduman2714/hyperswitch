mod cost_aware {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/router/src/core/routing/cost_aware.rs"
    ));
}

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use cost_aware::{select_connector, ConnectorCostConfig, CostRoutingConfig, RoutingDecision};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct CostAwareSelectRequest {
    payment_id: Option<String>,
    card_bin: String,
    currency: String,
    amount_in_usd: f64,
}

#[derive(Debug, Serialize)]
struct CostAwareSelectResponse {
    payment_id: String,
    routing_decision: RoutingDecision,
}

#[derive(Debug, Serialize)]
struct RoutingTraceResponse {
    payment_id: String,
    selected_connector: String,
    estimated_cost_usd: f64,
    reason: String,
    explanation: String,
    all_candidates: Vec<cost_aware::RoutingCandidate>,
}

fn connector(
    name: &str,
    base_fee_usd: f64,
    percent_fee: f64,
    currencies: &[&str],
    bin_prefixes: &[&str],
) -> ConnectorCostConfig {
    ConnectorCostConfig {
        name: name.to_string(),
        base_fee_usd,
        percent_fee,
        supported_currencies: currencies
            .iter()
            .map(|currency| currency.to_string())
            .collect(),
        supported_bin_prefixes: bin_prefixes
            .iter()
            .map(|prefix| prefix.to_string())
            .collect(),
    }
}

fn cost_config() -> CostRoutingConfig {
    CostRoutingConfig {
        connectors: vec![
            connector("stripe", 0.30, 0.029, &["USD", "EUR", "GBP"], &[]),
            connector("razorpay", 0.00, 0.020, &["INR"], &["508", "607"]),
            connector("adyen", 0.12, 0.025, &["USD", "EUR", "GBP", "INR"], &[]),
        ],
        min_success_rate: 0.80,
    }
}

fn response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn format_cost(cost: f64) -> String {
    format!("${cost:.2}")
}

fn build_trace_response(payment_id: String, decision: &RoutingDecision) -> RoutingTraceResponse {
    let candidate_summaries = decision
        .all_candidates
        .iter()
        .map(|candidate| {
            let floor_status = match &candidate.excluded_reason {
                Some(reason) => format!("excluded because {reason}"),
                None => "eligible".to_string(),
            };

            format!(
                "{} costs {} with success rate {:.2} and is {}",
                candidate.name,
                format_cost(candidate.estimated_cost_usd),
                candidate.success_rate,
                floor_status
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    let reason_text = match decision.reason.as_str() {
        "lowest_cost" => format!(
            "Selected {} because it was the cheapest connector among candidates that passed the success-rate floor.",
            decision.selected
        ),
        "floor_fallback" => format!(
            "Selected {} as the cheapest fallback because all candidates were below the success-rate floor.",
            decision.selected
        ),
        "only_option" => format!(
            "Selected {} because it was the only eligible connector.",
            decision.selected
        ),
        _ => format!("Selected {} because reason was {}.", decision.selected, decision.reason),
    };

    RoutingTraceResponse {
        payment_id,
        selected_connector: decision.selected.clone(),
        estimated_cost_usd: decision.estimated_cost_usd,
        reason: decision.reason.clone(),
        explanation: format!("{candidate_summaries}. {reason_text}"),
        all_candidates: decision.all_candidates.clone(),
    }
}

fn payment_id_from_trace_path(first_line: &str) -> Option<String> {
    let path = first_line.strip_prefix("GET ")?.split_whitespace().next()?;
    let payment_id = path
        .strip_prefix("/v1/payments/")?
        .strip_suffix("/routing-trace")?;

    (!payment_id.is_empty()).then(|| payment_id.to_string())
}

fn handle_connection(
    mut stream: TcpStream,
    trace_store: Arc<Mutex<HashMap<String, RoutingDecision>>>,
    payment_counter: Arc<AtomicUsize>,
) {
    let mut buffer = vec![0; 8192];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(bytes_read) => bytes_read,
        Err(_) => return,
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let Some((headers, body)) = request.split_once("\r\n\r\n") else {
        let _ = stream
            .write_all(response("400 Bad Request", r#"{"error":"invalid request"}"#).as_bytes());
        return;
    };

    let first_line = headers.lines().next().unwrap_or_default();
    if let Some(payment_id) = payment_id_from_trace_path(first_line) {
        let trace_store = match trace_store.lock() {
            Ok(trace_store) => trace_store,
            Err(_) => {
                let _ = stream.write_all(
                    response(
                        "500 Internal Server Error",
                        r#"{"error":"trace store unavailable"}"#,
                    )
                    .as_bytes(),
                );
                return;
            }
        };

        match trace_store.get(&payment_id) {
            Some(decision) => {
                let trace_response = build_trace_response(payment_id, decision);
                let body = serde_json::to_string_pretty(&trace_response).unwrap_or_else(|error| {
                    serde_json::json!({ "error": error.to_string() }).to_string()
                });
                let _ = stream.write_all(response("200 OK", &body).as_bytes());
            }
            None => {
                let body = serde_json::json!({
                    "error": "routing trace not found for this payment",
                    "payment_id": payment_id,
                })
                .to_string();
                let _ = stream.write_all(response("404 Not Found", &body).as_bytes());
            }
        }
        return;
    }

    if !headers.starts_with("POST /cost-aware/select ") {
        let _ = stream.write_all(response("404 Not Found", r#"{"error":"not found"}"#).as_bytes());
        return;
    }

    let payload = match serde_json::from_str::<CostAwareSelectRequest>(body) {
        Ok(payload) => payload,
        Err(error) => {
            let body = serde_json::json!({ "error": error.to_string() }).to_string();
            let _ = stream.write_all(response("400 Bad Request", &body).as_bytes());
            return;
        }
    };

    let config = cost_config();
    let result = select_connector(
        &config,
        &payload.card_bin,
        &payload.currency,
        payload.amount_in_usd,
    );

    match result {
        Ok(decision) => {
            let payment_id = payload.payment_id.unwrap_or_else(|| {
                let sequence = payment_counter.fetch_add(1, Ordering::Relaxed);
                format!("pay_cost_{sequence}")
            });

            if let Ok(mut trace_store) = trace_store.lock() {
                trace_store.insert(payment_id.clone(), decision.clone());
            }

            let response_body = CostAwareSelectResponse {
                payment_id,
                routing_decision: decision,
            };
            let body = serde_json::to_string_pretty(&response_body).unwrap_or_else(|error| {
                serde_json::json!({ "error": error.to_string() }).to_string()
            });
            let _ = stream.write_all(response("200 OK", &body).as_bytes());
        }
        Err(error) => {
            let body = serde_json::json!({ "error": error }).to_string();
            let _ = stream.write_all(response("400 Bad Request", &body).as_bytes());
        }
    }
}

fn main() {
    let port = std::env::var("COST_AWARE_PORT").unwrap_or_else(|_| "9091".to_string());
    let address = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&address).expect("server should bind to configured address");
    let trace_store = Arc::new(Mutex::new(HashMap::new()));
    let payment_counter = Arc::new(AtomicUsize::new(1));

    println!("Cost-aware demo API running at http://{address}");
    println!("POST /cost-aware/select");
    println!("GET  /v1/payments/{{payment_id}}/routing-trace");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(
                stream,
                Arc::clone(&trace_store),
                Arc::clone(&payment_counter),
            ),
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cost_aware::RoutingError;

    #[test]
    fn cost_aware_selects_stripe_when_adyen_is_below_success_floor() {
        let decision = select_connector(&cost_config(), "424242", "USD", 100.0)
            .expect("USD should have an eligible connector");

        assert_eq!(decision.selected, "stripe");
        assert_eq!(decision.reason, "lowest_cost");
        assert!(decision.all_candidates.iter().any(|candidate| {
            candidate.name == "adyen"
                && candidate.excluded_reason.as_deref() == Some("below_success_rate_floor")
        }));
    }

    #[test]
    fn cost_aware_selects_razorpay_for_inr_matching_bin() {
        let decision = select_connector(&cost_config(), "508999", "INR", 100.0)
            .expect("INR with 508 BIN should route");

        assert_eq!(decision.selected, "razorpay");
        assert_eq!(decision.reason, "lowest_cost");
    }

    #[test]
    fn cost_aware_falls_back_to_cheapest_when_all_are_below_floor() {
        let config = CostRoutingConfig {
            connectors: vec![
                connector("local_one", 0.20, 0.030, &["USD"], &[]),
                connector("local_two", 0.10, 0.020, &["USD"], &[]),
            ],
            min_success_rate: 0.95,
        };

        let decision = select_connector(&config, "424242", "USD", 100.0)
            .expect("fallback should still select a connector");

        assert_eq!(decision.selected, "local_two");
        assert_eq!(decision.reason, "floor_fallback");
    }

    #[test]
    fn cost_aware_returns_error_when_no_connector_supports_currency() {
        let result = select_connector(&cost_config(), "424242", "JPY", 100.0);

        assert!(matches!(result, Err(RoutingError::NoEligibleConnector(_))));
    }
}

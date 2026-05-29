use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCostConfig {
    pub name: String,
    pub base_fee_usd: f64,
    pub percent_fee: f64,
    pub supported_currencies: Vec<String>,
    pub supported_bin_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRoutingConfig {
    pub connectors: Vec<ConnectorCostConfig>,
    pub min_success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingCandidate {
    pub name: String,
    pub estimated_cost_usd: f64,
    pub success_rate: f64,
    pub excluded_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub selected: String,
    pub estimated_cost_usd: f64,
    pub reason: String,
    pub all_candidates: Vec<RoutingCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingError {
    NoEligibleConnector(String),
    ConfigLoadError(String),
}

pub fn calculate_estimated_cost_usd(
    base_fee_usd: f64,
    percent_fee: f64,
    amount_in_usd: f64,
) -> f64 {
    base_fee_usd + (percent_fee * amount_in_usd)
}

pub fn bin_matches_connector(connector: &ConnectorCostConfig, card_bin: &str) -> bool {
    connector.supported_bin_prefixes.is_empty()
        || connector
            .supported_bin_prefixes
            .iter()
            .any(|prefix| card_bin.starts_with(prefix))
}

pub fn connector_supports_currency(connector: &ConnectorCostConfig, currency: &str) -> bool {
    connector
        .supported_currencies
        .iter()
        .any(|supported_currency| supported_currency.eq_ignore_ascii_case(currency))
}

pub fn mock_success_rate(connector_name: &str) -> f64 {
    match connector_name.to_ascii_lowercase().as_str() {
        "stripe" => 0.95,
        "razorpay" => 0.88,
        "adyen" => 0.72,
        _ => 0.85,
    }
}

pub fn select_connector(
    config: &CostRoutingConfig,
    card_bin: &str,
    currency: &str,
    amount_in_usd: f64,
) -> Result<RoutingDecision, RoutingError> {
    let eligible_connectors = config
        .connectors
        .iter()
        .filter(|connector| connector_supports_currency(connector, currency))
        .filter(|connector| bin_matches_connector(connector, card_bin))
        .collect::<Vec<_>>();

    if eligible_connectors.is_empty() {
        return Err(RoutingError::NoEligibleConnector(format!(
            "No connector supports currency {currency} and BIN {card_bin}"
        )));
    }

    let all_candidates = eligible_connectors
        .into_iter()
        .map(|connector| {
            let estimated_cost_usd = calculate_estimated_cost_usd(
                connector.base_fee_usd,
                connector.percent_fee,
                amount_in_usd,
            );
            let success_rate = mock_success_rate(&connector.name);
            let excluded_reason = (success_rate < config.min_success_rate)
                .then(|| "below_success_rate_floor".to_string());

            RoutingCandidate {
                name: connector.name.clone(),
                estimated_cost_usd,
                success_rate,
                excluded_reason,
            }
        })
        .collect::<Vec<_>>();

    let passing_floor = all_candidates
        .iter()
        .filter(|candidate| candidate.success_rate >= config.min_success_rate)
        .collect::<Vec<_>>();

    let (selected_candidate, reason) = if let Some(candidate) = passing_floor
        .iter()
        .min_by(|left, right| left.estimated_cost_usd.total_cmp(&right.estimated_cost_usd))
    {
        (*candidate, "lowest_cost")
    } else {
        let fallback_candidate = all_candidates
            .iter()
            .min_by(|left, right| left.estimated_cost_usd.total_cmp(&right.estimated_cost_usd))
            .ok_or_else(|| {
                RoutingError::NoEligibleConnector(format!(
                    "No connector supports currency {currency} and BIN {card_bin}"
                ))
            })?;

        (fallback_candidate, "floor_fallback")
    };

    Ok(RoutingDecision {
        selected: selected_candidate.name.clone(),
        estimated_cost_usd: selected_candidate.estimated_cost_usd,
        reason: reason.to_string(),
        all_candidates,
    })
}

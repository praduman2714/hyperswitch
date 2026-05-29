use router::core::routing::cost_aware::{
    select_connector, ConnectorCostConfig, CostRoutingConfig, RoutingError,
};

fn connector(
    name: &str,
    base_fee_usd: f64,
    percent_fee: f64,
    supported_currencies: &[&str],
    supported_bin_prefixes: &[&str],
) -> ConnectorCostConfig {
    ConnectorCostConfig {
        name: name.to_string(),
        base_fee_usd,
        percent_fee,
        supported_currencies: supported_currencies
            .iter()
            .map(|currency| currency.to_string())
            .collect(),
        supported_bin_prefixes: supported_bin_prefixes
            .iter()
            .map(|prefix| prefix.to_string())
            .collect(),
    }
}

#[test]
fn cost_aware_selects_cheapest_connector_above_success_floor() {
    let config = CostRoutingConfig {
        connectors: vec![
            connector("stripe", 0.30, 0.029, &["USD"], &[]),
            connector("adyen", 0.00, 0.010, &["USD"], &[]),
        ],
        min_success_rate: 0.80,
    };

    let decision = select_connector(&config, "424242", "USD", 100.0)
        .expect("stripe should be selected because adyen is below the floor");

    assert_eq!(decision.selected, "stripe");
    assert_eq!(decision.reason, "lowest_cost");
    assert_eq!(decision.all_candidates.len(), 2);

    let adyen = decision
        .all_candidates
        .iter()
        .find(|candidate| candidate.name == "adyen")
        .expect("adyen should be present in the audit trail");
    assert_eq!(
        adyen.excluded_reason.as_deref(),
        Some("below_success_rate_floor")
    );
}

#[test]
fn cost_aware_selects_razorpay_for_matching_inr_bin() {
    let config = CostRoutingConfig {
        connectors: vec![
            connector("razorpay", 0.00, 0.020, &["INR"], &["508", "607"]),
            connector("adyen", 0.12, 0.025, &["INR"], &[]),
        ],
        min_success_rate: 0.80,
    };

    let decision = select_connector(&config, "508999", "INR", 100.0)
        .expect("razorpay should be selected for matching INR BIN");

    assert_eq!(decision.selected, "razorpay");
    assert_eq!(decision.reason, "lowest_cost");
}

#[test]
fn cost_aware_falls_back_to_cheapest_when_all_connectors_are_below_floor() {
    let config = CostRoutingConfig {
        connectors: vec![
            connector("local_one", 0.10, 0.030, &["USD"], &[]),
            connector("local_two", 0.20, 0.010, &["USD"], &[]),
        ],
        min_success_rate: 0.95,
    };

    let decision = select_connector(&config, "424242", "USD", 100.0)
        .expect("routing should fall back instead of hard blocking");

    assert_eq!(decision.selected, "local_two");
    assert_eq!(decision.reason, "floor_fallback");
    assert!(decision
        .all_candidates
        .iter()
        .all(|candidate| candidate.excluded_reason.as_deref() == Some("below_success_rate_floor")));
}

#[test]
fn cost_aware_returns_no_eligible_connector_for_unsupported_currency() {
    let config = CostRoutingConfig {
        connectors: vec![
            connector("stripe", 0.30, 0.029, &["USD"], &[]),
            connector("razorpay", 0.00, 0.020, &["INR"], &["508", "607"]),
        ],
        min_success_rate: 0.80,
    };

    let result = select_connector(&config, "424242", "JPY", 100.0);

    assert!(matches!(result, Err(RoutingError::NoEligibleConnector(_))));
}

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

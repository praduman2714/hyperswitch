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

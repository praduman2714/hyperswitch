use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use super::cost_aware::RoutingDecision;

#[derive(Debug, Clone)]
pub struct CostAwareRoutingStore {
    decisions: Arc<Mutex<HashMap<String, RoutingDecision>>>,
}

impl CostAwareRoutingStore {
    pub fn new() -> Self {
        Self {
            decisions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn insert(&self, payment_id: String, decision: RoutingDecision) -> Result<(), String> {
        let mut decisions = self
            .decisions
            .lock()
            .map_err(|error| format!("cost-aware routing store lock poisoned: {error}"))?;

        decisions.insert(payment_id, decision);
        Ok(())
    }

    pub fn get(&self, payment_id: &str) -> Result<Option<RoutingDecision>, String> {
        let decisions = self
            .decisions
            .lock()
            .map_err(|error| format!("cost-aware routing store lock poisoned: {error}"))?;

        Ok(decisions.get(payment_id).cloned())
    }
}

impl Default for CostAwareRoutingStore {
    fn default() -> Self {
        Self::new()
    }
}

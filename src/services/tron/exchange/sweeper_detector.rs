use std::collections::HashMap;

use crate::services::tron::aml::types::SimpleTransfer;

pub fn detect_sweep_wallets(transfers: &[SimpleTransfer]) -> Vec<String> {
    let mut inbound = HashMap::<String, usize>::new();

    for t in transfers {
        *inbound.entry(t.to.clone()).or_insert(0) += 1;
    }

    inbound
        .into_iter()
        .filter(|(_, count)| *count >= 10)
        .map(|(wallet, _)| wallet)
        .collect()
}

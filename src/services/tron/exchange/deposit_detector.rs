use std::collections::HashMap;

use crate::services::tron::aml::types::SimpleTransfer;

pub fn detect_exchange_deposits(transfers: &[SimpleTransfer]) -> HashMap<String, String> {
    //
    // deposit addr -> exchange
    //

    let mut results = HashMap::new();

    //
    // heuristic:
    //
    // many deposit wallets
    // sweep into same wallet
    //

    let mut sweep_targets = HashMap::<String, usize>::new();

    for t in transfers {
        *sweep_targets.entry(t.to.clone()).or_insert(0) += 1;
    }

    for (wallet, count) in sweep_targets {
        if count >= 5 {
            results.insert(wallet, "UnknownExchange".to_string());
        }
    }

    results
}

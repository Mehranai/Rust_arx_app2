use std::collections::HashMap;

use crate::services::tron::aml::types::SimpleTransfer;

// many wallets -> one wallet
pub fn is_probable_sweeper(transfers: &[SimpleTransfer]) -> bool {
    let mut inbound = HashMap::<String, usize>::new();

    for t in transfers {
        *inbound.entry(t.to.clone()).or_insert(0) += 1;
    }
    inbound.values().any(|count| *count >= 5)
}
// one wallet -> many users

pub fn is_probable_withdrawal_wallet(transfers: &[SimpleTransfer]) -> bool {
    let mut outbound = HashMap::<String, usize>::new();

    for t in transfers {
        *outbound.entry(t.from.clone()).or_insert(0) += 1;
    }
    outbound.values().any(|count| *count >= 5)
}

use crate::services::tron::aml::types::SimpleTransfer;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ClusterExpansionResult {
    pub cluster_id: String,
    pub discovered_wallets: HashSet<String>,
    pub sweeper_wallets: HashSet<String>,
    pub deposit_wallets: HashSet<String>,
}

pub fn expand_exchange_cluster(
    seed_wallet: &str,
    transfers: &[SimpleTransfer],
) -> ClusterExpansionResult {
    let mut discovered_wallets = HashSet::<String>::new();

    let mut sweeper_wallets = HashSet::<String>::new();

    let mut deposit_wallets = HashSet::<String>::new();

    //
    // deposit -> sweeper frequency
    //
    // many wallets
    // sending into
    // same wallet
    //

    let mut inbound_map = HashMap::<String, Vec<String>>::new();

    for t in transfers {
        inbound_map
            .entry(t.to.clone())
            .or_default()
            .push(t.from.clone());
    }

    for (receiver, senders) in inbound_map {
        let unique_senders = senders.iter().cloned().collect::<HashSet<_>>();

        //
        // probable exchange sweeper
        //

        if unique_senders.len() >= 10 {
            sweeper_wallets.insert(receiver.clone());

            discovered_wallets.insert(receiver.clone());

            for sender in unique_senders {
                deposit_wallets.insert(sender.clone());

                discovered_wallets.insert(sender);
            }
        }
    }

    //
    // seed wallet neighbors
    //

    for t in transfers {
        if t.from == seed_wallet {
            discovered_wallets.insert(t.to.clone());
        }

        if t.to == seed_wallet {
            discovered_wallets.insert(t.from.clone());
        }
    }

    ClusterExpansionResult {
        cluster_id: format!("cluster:{}", seed_wallet),

        discovered_wallets,
        sweeper_wallets,
        deposit_wallets,
    }
}

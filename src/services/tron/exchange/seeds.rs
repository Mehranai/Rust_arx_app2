use std::collections::HashMap;

use super::types::{ExchangeAttribution, ExchangeWalletRole};

pub fn exchange_seeds() -> HashMap<String, ExchangeAttribution> {
    let mut map = HashMap::new();
    //
    // Binance examples
    //
    map.insert(
        "TAUN6FwrnwwmaEqYcckffC7wYmbaS6cBiX".to_string(),
        ExchangeAttribution {
            exchange_name: "Binance".to_string(),

            role: ExchangeWalletRole::Hot.to_string(),

            confidence: 1.0,

            detection_source: "seed".to_string(),

            cluster_id: Some("binance_tron".to_string()),
        },
    );

    //
    // OKX examples
    //

    map.insert(
        "TU2TmqauSEiRf16CyFgzHV2BVxBejY9iyR".to_string(),
        ExchangeAttribution {
            exchange_name: "OKX".to_string(),

            role: ExchangeWalletRole::Hot.to_string(),

            confidence: 1.0,

            detection_source: "seed".to_string(),

            cluster_id: Some("okx_tron".to_string()),
        },
    );

    map
}

use std::collections::HashMap;

pub const ZERO_ADDRESS: &str = "T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb";

#[derive(Debug, Clone)]
pub struct SimpleTransfer {
    pub token: String,
    pub from: String,
    pub to: String,
    pub amount: u128,
}

#[derive(Debug, Clone)]
pub struct NetFlow {
    pub address: String,
    pub token: String,
    pub delta: i128,
}

#[derive(Debug, Clone)]
pub enum AmlEvent {
    Swap {
        user: String,
        token_in: String,
        token_out: String,
    },

    BridgeIn {
        user: String,
        token: String,
    },

    BridgeOut {
        user: String,
        token: String,
    },

    Mint {
        user: String,
        token: String,
    },

    Burn {
        user: String,
        token: String,
    },
}

pub type FlowMap = HashMap<String, HashMap<String, i128>>;

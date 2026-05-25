use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractCategory {
    Dex,
    Bridge,
    Lending,
    Staking,
    Mixer,
    Token,
    Nft,
    Scam,
    Wallet,
    Unknown,
}

impl ToString for ContractCategory {
    fn to_string(&self) -> String {
        match self {
            Self::Dex => "DEX",
            Self::Bridge => "BRIDGE",
            Self::Lending => "LENDING",
            Self::Staking => "STAKING",
            Self::Mixer => "MIXER",
            Self::Token => "TOKEN",
            Self::Nft => "NFT",
            Self::Scam => "SCAM",
            Self::Wallet => "WALLET",
            Self::Unknown => "UNKNOWN",
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ClassificationInput {
    pub contract_address: String,

    pub method_data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub protocol: &'static str,

    pub category: ContractCategory,

    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub protocol: String,

    pub category: ContractCategory,

    pub confidence: f32,

    pub detection_source: String,

    pub method_id: Option<String>,
}

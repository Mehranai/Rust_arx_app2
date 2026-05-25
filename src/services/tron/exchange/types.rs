#[derive(Debug, Clone)]

pub enum ExchangeWalletRole {
    Hot,
    Deposit,
    Sweep,
    Treasury,
    Withdraw,
    Internal,
    Unknown,
}

impl ToString for ExchangeWalletRole {
    fn to_string(&self) -> String {
        match self {
            Self::Hot => "HOT",

            Self::Deposit => "DEPOSIT",

            Self::Sweep => "SWEEP",

            Self::Treasury => "TREASURY",

            Self::Withdraw => "WITHDRAW",

            Self::Internal => "INTERNAL",

            Self::Unknown => "UNKNOWN",
        }
        .to_string()
    }
}

#[derive(Debug, Clone)]

pub struct ExchangeAttribution {
    pub exchange_name: String,
    pub role: String,
    pub confidence: f32,
    pub detection_source: String,
    pub cluster_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExchangeClusterMember {
    pub cluster_id: String,
    pub address: String,
    pub role: String,
    pub confidence: f32,
}

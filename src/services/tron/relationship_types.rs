#[derive(Debug, Clone)]
pub enum RelationshipType {
    NativeTransfer,

    Trc20Transfer,

    Swap,

    Bridge,

    ExchangeDeposit,

    ExchangeWithdraw,

    LiquidityAdd,

    LiquidityRemove,

    InternalTransfer,
}

impl ToString for RelationshipType {
    fn to_string(&self) -> String {
        match self {
            Self::NativeTransfer => "native_transfer",

            Self::Trc20Transfer => "trc20_transfer",

            Self::Swap => "swap",

            Self::Bridge => "bridge",

            Self::ExchangeDeposit => "exchange_deposit",

            Self::ExchangeWithdraw => "exchange_withdraw",

            Self::LiquidityAdd => "liquidity_add",

            Self::LiquidityRemove => "liquidity_remove",

            Self::InternalTransfer => "internal_transfer",
        }
        .to_string()
    }
}

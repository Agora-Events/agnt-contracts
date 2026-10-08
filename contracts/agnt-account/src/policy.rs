use soroban_sdk::{contracttype, Address, BytesN, Vec};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    pub per_tx_cap: i128,
    pub daily_cap: i128,
    pub payees: Vec<Address>,
    pub expires_at: u64,
    pub revoked: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendTracker {
    pub day: u64,
    pub spent: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

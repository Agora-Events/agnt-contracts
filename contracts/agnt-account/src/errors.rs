use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum AgntError {
    NotOwner = 1,
    UnknownSigner = 2,
    AgentRevoked = 3,
    PolicyExpired = 4,
    ContextNotAllowed = 5,
    PayeeNotAllowed = 6,
    OverPerTxCap = 7,
    OverDailyCap = 8,
    InvalidAmount = 9,
    BadSignature = 10,
}

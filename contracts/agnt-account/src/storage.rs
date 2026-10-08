use soroban_sdk::{contracttype, Address, BytesN, Env};
use crate::policy::{Policy, SpendTracker};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Owner,
    Usdc,
    Agent(BytesN<32>),
    SpentToday(BytesN<32>),
}

pub fn set_owner(env: &Env, owner: &BytesN<32>) {
    env.storage().instance().set(&DataKey::Owner, owner);
}

pub fn get_owner(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&DataKey::Owner)
}

pub fn set_usdc(env: &Env, usdc: &Address) {
    env.storage().instance().set(&DataKey::Usdc, usdc);
}

pub fn get_usdc(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Usdc)
}

pub fn set_agent_policy(env: &Env, agent: &BytesN<32>, policy: &Policy) {
    env.storage().persistent().set(&DataKey::Agent(agent.clone()), policy);
}

pub fn get_agent_policy(env: &Env, agent: &BytesN<32>) -> Option<Policy> {
    env.storage().persistent().get(&DataKey::Agent(agent.clone()))
}

pub fn get_spend_tracker(env: &Env, agent: &BytesN<32>) -> Option<SpendTracker> {
    env.storage().persistent().get(&DataKey::SpentToday(agent.clone()))
}

pub fn set_spend_tracker(env: &Env, agent: &BytesN<32>, tracker: &SpendTracker) {
    env.storage().persistent().set(&DataKey::SpentToday(agent.clone()), tracker);
}

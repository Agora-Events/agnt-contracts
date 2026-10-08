#![no_std]

pub mod errors;
pub mod events;
pub mod policy;
pub mod storage;

#[cfg(test)]
mod test;

use soroban_sdk::{
    auth::{ContractContext, CustomAccountInterface, Context},
    contract, contractimpl, Address, BytesN, Env, Hash, IntoVal, Symbol, TryFromVal, Vec,
};

use errors::AgntError;
use events::*;
use policy::{Policy, Signature, SpendTracker};
use storage::*;

#[contract]
pub struct AgntAccount;

#[contractimpl]
impl AgntAccount {
    pub fn __constructor(env: Env, owner: BytesN<32>, usdc: Address) {
        set_owner(&env, &owner);
        set_usdc(&env, &usdc);
    }

    pub fn add_agent(env: Env, agent: BytesN<32>, policy: Policy) {
        env.current_contract_address().require_auth();
        set_agent_policy(&env, &agent, &policy);
        emit_agent_added(&env, agent);
    }

    pub fn update_policy(env: Env, agent: BytesN<32>, policy: Policy) {
        env.current_contract_address().require_auth();
        if get_agent_policy(&env, &agent).is_none() {
            panic!("agent does not exist");
        }
        set_agent_policy(&env, &agent, &policy);
        emit_agent_updated(&env, agent);
    }

    pub fn revoke_agent(env: Env, agent: BytesN<32>) {
        env.current_contract_address().require_auth();
        let mut policy = get_agent_policy(&env, &agent).expect("agent does not exist");
        policy.revoked = true;
        set_agent_policy(&env, &agent, &policy);
        emit_agent_revoked(&env, agent);
    }

    pub fn get_agent(env: Env, agent: BytesN<32>) -> Policy {
        get_agent_policy(&env, &agent).expect("agent does not exist")
    }

    pub fn get_spent_today(env: Env, agent: BytesN<32>) -> i128 {
        let current_day = env.ledger().timestamp() / 86400;
        match get_spend_tracker(&env, &agent) {
            Some(tracker) => {
                if tracker.day == current_day {
                    tracker.spent
                } else {
                    0
                }
            }
            None => 0,
        }
    }
}

#[contractimpl]
impl CustomAccountInterface for AgntAccount {
    type Error = AgntError;
    type Signature = Signature;

    fn __check_auth(
        env: Env,
        signature_payload: Hash<32>,
        signature: Signature,
        auth_contexts: Vec<Context>,
    ) -> Result<(), AgntError> {
        env.crypto().ed25519_verify(
            &signature.public_key,
            &signature_payload.to_bytes().into(),
            &signature.signature,
        );

        let owner = get_owner(&env).ok_or(AgntError::NotOwner)?;
        if signature.public_key == owner {
            return Ok(());
        }

        let agent_pk = signature.public_key;
        let policy = get_agent_policy(&env, &agent_pk).ok_or(AgntError::UnknownSigner)?;

        if policy.revoked {
            return Err(AgntError::AgentRevoked);
        }

        if env.ledger().timestamp() >= policy.expires_at {
            return Err(AgntError::PolicyExpired);
        }

        let usdc = get_usdc(&env).ok_or(AgntError::ContextNotAllowed)?;
        let current_day = env.ledger().timestamp() / 86400;

        let mut tracker = match get_spend_tracker(&env, &agent_pk) {
            Some(t) if t.day == current_day => t,
            _ => SpendTracker {
                day: current_day,
                spent: 0,
            },
        };

        let current_contract = env.current_contract_address();

        for context in auth_contexts.iter() {
            let ContractContext {
                contract,
                fn_name,
                args,
            } = match context {
                Context::Contract(c) => c,
                Context::CreateContractHostFn(_) => return Err(AgntError::ContextNotAllowed),
            };

            if contract != usdc {
                return Err(AgntError::ContextNotAllowed);
            }

            if fn_name != Symbol::new(&env, "transfer") {
                return Err(AgntError::ContextNotAllowed);
            }

            if args.len() != 3 {
                return Err(AgntError::ContextNotAllowed);
            }

            let from: Address = match args.get(0).unwrap().try_into_val(&env) {
                Ok(a) => a,
                Err(_) => return Err(AgntError::ContextNotAllowed),
            };

            let to: Address = match args.get(1).unwrap().try_into_val(&env) {
                Ok(a) => a,
                Err(_) => return Err(AgntError::ContextNotAllowed),
            };

            let amount: i128 = match args.get(2).unwrap().try_into_val(&env) {
                Ok(a) => a,
                Err(_) => return Err(AgntError::ContextNotAllowed),
            };

            if from != current_contract {
                return Err(AgntError::ContextNotAllowed);
            }

            if !policy.payees.contains(&to) {
                return Err(AgntError::PayeeNotAllowed);
            }

            if amount <= 0 {
                return Err(AgntError::InvalidAmount);
            }

            if amount > policy.per_tx_cap {
                return Err(AgntError::OverPerTxCap);
            }

            let new_spent = tracker
                .spent
                .checked_add(amount)
                .ok_or(AgntError::OverDailyCap)?;

            if new_spent > policy.daily_cap {
                return Err(AgntError::OverDailyCap);
            }

            tracker.spent = new_spent;
            emit_payment_approved(&env, agent_pk.clone(), to, amount);
        }

        set_spend_tracker(&env, &agent_pk, &tracker);

        Ok(())
    }
}

use crate::{
    errors::AgntError,
    policy::{Policy, Signature},
    AgntAccount, AgntAccountClient,
};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    auth::{ContractContext, Context},
    testutils::{Address as _, BytesN as _, Ledger},
    Address, BytesN, Env, Hash, IntoVal, Symbol, Vec,
};

fn create_signer(env: &Env) -> (SigningKey, BytesN<32>) {
    let key = SigningKey::generate(&mut OsRng);
    let pk = BytesN::from_array(env, &key.verifying_key().to_bytes());
    (key, pk)
}

fn sign_payload(env: &Env, key: &SigningKey, pk: &BytesN<32>, payload: &Hash<32>) -> Signature {
    let payload_bytes = payload.to_array();
    let sig_dalek = key.sign(&payload_bytes);
    let sig_bytes = BytesN::from_array(env, &sig_dalek.to_bytes());
    Signature {
        public_key: pk.clone(),
        signature: sig_bytes,
    }
}

fn setup_test() -> (
    Env,
    Address,
    AgntAccountClient<'static>,
    SigningKey,
    BytesN<32>,
    Address,
    SigningKey,
    BytesN<32>,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let (owner_key, owner_pk) = create_signer(&env);
    let usdc = Address::generate(&env);
    let payee = Address::generate(&env);

    let (agent_key, agent_pk) = create_signer(&env);

    let contract_id = env.register(AgntAccount, (&owner_pk, &usdc));
    let client = AgntAccountClient::new(&env, &contract_id);

    (
        env,
        contract_id,
        client,
        owner_key,
        owner_pk,
        usdc,
        agent_key,
        agent_pk,
        payee,
    )
}

#[test]
fn test_agent_payment_within_limits_passes() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let policy = Policy {
        per_tx_cap: 100_000_000, // 10 USDC (7 decimals)
        daily_cap: 500_000_000,  // 50 USDC
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };

    client.add_agent(&agent_pk, &policy);

    let payload = Hash::from_array(&env, &[1u8; 32]);
    let sig = sign_payload(&env, &agent_key, &agent_pk, &payload);

    let context = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 50_000_000i128).into_val(&env),
    });

    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload,
        &sig,
        &Vec::from_array(&env, [context]),
    );

    assert!(res.is_ok());
    assert_eq!(client.get_spent_today(&agent_pk), 50_000_000i128);
}

#[test]
fn test_over_per_tx_cap_fails() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    let payload = Hash::from_array(&env, &[2u8; 32]);
    let sig = sign_payload(&env, &agent_key, &agent_pk, &payload);

    let context = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee, 150_000_000i128).into_val(&env),
    });

    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload,
        &sig,
        &Vec::from_array(&env, [context]),
    );

    assert_eq!(res, Err(Ok(AgntError::OverPerTxCap)));
}

#[test]
fn test_cumulative_spend_daily_cap_and_day_reset() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    env.ledger().set_timestamp(100);

    let policy = Policy {
        per_tx_cap: 300_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    let payload1 = Hash::from_array(&env, &[3u8; 32]);
    let sig1 = sign_payload(&env, &agent_key, &agent_pk, &payload1);
    let context1 = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 300_000_000i128).into_val(&env),
    });

    let res1 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload1,
        &sig1,
        &Vec::from_array(&env, [context1]),
    );
    assert!(res1.is_ok());
    assert_eq!(client.get_spent_today(&agent_pk), 300_000_000i128);

    // Second payment: 300 stroops -> exceeds 500 stroops daily cap (total 600)
    let payload2 = Hash::from_array(&env, &[4u8; 32]);
    let sig2 = sign_payload(&env, &agent_key, &agent_pk, &payload2);
    let context2 = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 300_000_000i128).into_val(&env),
    });

    let res2 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload2,
        &sig2,
        &Vec::from_array(&env, [context2]),
    );
    assert_eq!(res2, Err(Ok(AgntError::OverDailyCap)));

    // Advance timestamp past day boundary (86400 seconds)
    env.ledger().set_timestamp(100 + 86400);

    assert_eq!(client.get_spent_today(&agent_pk), 0i128);

    // Payment should now pass again
    let payload3 = Hash::from_array(&env, &[5u8; 32]);
    let sig3 = sign_payload(&env, &agent_key, &agent_pk, &payload3);
    let context3 = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee, 300_000_000i128).into_val(&env),
    });

    let res3 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload3,
        &sig3,
        &Vec::from_array(&env, [context3]),
    );
    assert!(res3.is_ok());
    assert_eq!(client.get_spent_today(&agent_pk), 300_000_000i128);
}

#[test]
fn test_payee_not_in_allowlist_fails() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let unallowed_payee = Address::generate(&env);

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    let payload = Hash::from_array(&env, &[6u8; 32]);
    let sig = sign_payload(&env, &agent_key, &agent_pk, &payload);

    let context = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), unallowed_payee, 50_000_000i128).into_val(&env),
    });

    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload,
        &sig,
        &Vec::from_array(&env, [context]),
    );

    assert_eq!(res, Err(Ok(AgntError::PayeeNotAllowed)));
}

#[test]
fn test_expired_policy_fails() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    env.ledger().set_timestamp(2000);

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1500, // Expired
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    let payload = Hash::from_array(&env, &[7u8; 32]);
    let sig = sign_payload(&env, &agent_key, &agent_pk, &payload);

    let context = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee, 50_000_000i128).into_val(&env),
    });

    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload,
        &sig,
        &Vec::from_array(&env, [context]),
    );

    assert_eq!(res, Err(Ok(AgntError::PolicyExpired)));
}

#[test]
fn test_revoked_agent_fails_and_mid_sequence_revocation() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    // First payment succeeds
    let payload1 = Hash::from_array(&env, &[8u8; 32]);
    let sig1 = sign_payload(&env, &agent_key, &agent_pk, &payload1);
    let context1 = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 10_000_000i128).into_val(&env),
    });
    assert!(env
        .try_invoke_contract_check_auth::<AgntError>(
            &contract_id,
            &payload1,
            &sig1,
            &Vec::from_array(&env, [context1])
        )
        .is_ok());

    // Owner revokes agent
    client.revoke_agent(&agent_pk);
    assert!(client.get_agent(&agent_pk).revoked);

    // Second payment fails
    let payload2 = Hash::from_array(&env, &[9u8; 32]);
    let sig2 = sign_payload(&env, &agent_key, &agent_pk, &payload2);
    let context2 = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee, 10_000_000i128).into_val(&env),
    });

    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload2,
        &sig2,
        &Vec::from_array(&env, [context2]),
    );
    assert_eq!(res, Err(Ok(AgntError::AgentRevoked)));
}

#[test]
fn test_invalid_contexts_fail() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    // 1. Non-transfer fn
    let payload1 = Hash::from_array(&env, &[10u8; 32]);
    let sig1 = sign_payload(&env, &agent_key, &agent_pk, &payload1);
    let context_bad_fn = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "approve"),
        args: (contract_id.clone(), payee.clone(), 10_000_000i128).into_val(&env),
    });
    let res1 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload1,
        &sig1,
        &Vec::from_array(&env, [context_bad_fn]),
    );
    assert_eq!(res1, Err(Ok(AgntError::ContextNotAllowed)));

    // 2. Transfer on different token contract
    let other_token = Address::generate(&env);
    let payload2 = Hash::from_array(&env, &[11u8; 32]);
    let sig2 = sign_payload(&env, &agent_key, &agent_pk, &payload2);
    let context_bad_token = Context::Contract(ContractContext {
        contract: other_token,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 10_000_000i128).into_val(&env),
    });
    let res2 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload2,
        &sig2,
        &Vec::from_array(&env, [context_bad_token]),
    );
    assert_eq!(res2, Err(Ok(AgntError::ContextNotAllowed)));

    // 3. Transfer with from != this account
    let other_sender = Address::generate(&env);
    let payload3 = Hash::from_array(&env, &[12u8; 32]);
    let sig3 = sign_payload(&env, &agent_key, &agent_pk, &payload3);
    let context_bad_from = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (other_sender, payee, 10_000_000i128).into_val(&env),
    });
    let res3 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload3,
        &sig3,
        &Vec::from_array(&env, [context_bad_from]),
    );
    assert_eq!(res3, Err(Ok(AgntError::ContextNotAllowed)));
}

#[test]
fn test_wrong_signature_and_unknown_signer_fail() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    // 1. Unknown signer
    let (unregistered_key, unregistered_pk) = create_signer(&env);
    let payload1 = Hash::from_array(&env, &[13u8; 32]);
    let sig_unregistered = sign_payload(&env, &unregistered_key, &unregistered_pk, &payload1);
    let context = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 10_000_000i128).into_val(&env),
    });
    let res1 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload1,
        &sig_unregistered,
        &Vec::from_array(&env, [context.clone()]),
    );
    assert_eq!(res1, Err(Ok(AgntError::UnknownSigner)));

    // 2. Wrong signature (signature for payload A attached to payload B)
    let payload_b = Hash::from_array(&env, &[14u8; 32]);
    let sig_wrong = sign_payload(&env, &agent_key, &agent_pk, &payload1); // signed payload1 instead of payload_b

    let res2 = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload_b,
        &sig_wrong,
        &Vec::from_array(&env, [context]),
    );
    assert!(res2.is_err()); // Host panic on ed25519_verify failure
}

#[test]
fn test_two_transfers_in_one_invocation_count_cumulatively() {
    let (env, contract_id, client, _, _, usdc, agent_key, agent_pk, payee) = setup_test();

    let payee2 = Address::generate(&env);

    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 150_000_000, // Daily cap is 15 USDC
        payees: Vec::from_array(&env, [payee.clone(), payee2.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);

    // Multi-context invocation: 9 USDC + 8 USDC = 17 USDC -> Exceeds daily cap 15 USDC
    let payload = Hash::from_array(&env, &[15u8; 32]);
    let sig = sign_payload(&env, &agent_key, &agent_pk, &payload);

    let context1 = Context::Contract(ContractContext {
        contract: usdc.clone(),
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee.clone(), 90_000_000i128).into_val(&env),
    });
    let context2 = Context::Contract(ContractContext {
        contract: usdc,
        fn_name: Symbol::new(&env, "transfer"),
        args: (contract_id.clone(), payee2, 80_000_000i128).into_val(&env),
    });

    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload,
        &sig,
        &Vec::from_array(&env, [context1, context2]),
    );
    assert_eq!(res, Err(Ok(AgntError::OverDailyCap)));

    // Ensure state was not permanently mutated on failure (spend is still 0)
    assert_eq!(client.get_spent_today(&agent_pk), 0i128);
}

#[test]
fn test_owner_auth_and_management() {
    let (env, contract_id, client, owner_key, owner_pk, _, _, agent_pk, payee) = setup_test();

    // Owner checks auth passes for any context
    let payload = Hash::from_array(&env, &[16u8; 32]);
    let owner_sig = sign_payload(&env, &owner_key, &owner_pk, &payload);
    let res = env.try_invoke_contract_check_auth::<AgntError>(
        &contract_id,
        &payload,
        &owner_sig,
        &Vec::new(&env),
    );
    assert!(res.is_ok());

    // Owner adds agent
    let policy = Policy {
        per_tx_cap: 100_000_000,
        daily_cap: 500_000_000,
        payees: Vec::from_array(&env, [payee.clone()]),
        expires_at: 1000000,
        revoked: false,
    };
    client.add_agent(&agent_pk, &policy);
    assert_eq!(client.get_agent(&agent_pk), policy);

    // Owner updates agent policy
    let updated_policy = Policy {
        per_tx_cap: 200_000_000,
        daily_cap: 800_000_000,
        payees: Vec::from_array(&env, [payee]),
        expires_at: 2000000,
        revoked: false,
    };
    client.update_policy(&agent_pk, &updated_policy);
    assert_eq!(client.get_agent(&agent_pk), updated_policy);

    // Owner revokes agent
    client.revoke_agent(&agent_pk);
    assert!(client.get_agent(&agent_pk).revoked);
}

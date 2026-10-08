use soroban_sdk::{symbol_short, Address, BytesN, Env};

pub fn emit_agent_added(env: &Env, agent: BytesN<32>) {
    env.events().publish(
        (symbol_short!("agnt_add"), agent),
        (),
    );
}

pub fn emit_agent_updated(env: &Env, agent: BytesN<32>) {
    env.events().publish(
        (symbol_short!("agnt_upd"), agent),
        (),
    );
}

pub fn emit_agent_revoked(env: &Env, agent: BytesN<32>) {
    env.events().publish(
        (symbol_short!("agnt_rev"), agent),
        (),
    );
}

pub fn emit_payment_approved(env: &Env, agent: BytesN<32>, to: Address, amount: i128) {
    env.events().publish(
        (symbol_short!("pay_appr"), agent),
        (to, amount),
    );
}

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum MockGuardKey {
    Maintainer(Address),
}

#[contract]
pub struct MockWaveGuard;

#[contractimpl]
impl MockWaveGuard {
    pub fn is_maintainer(env: Env, address: Address) -> bool {
        env.storage().instance().get::<_, bool>(&MockGuardKey::Maintainer(address)).unwrap_or(false)
    }

    pub fn add_maintainer(env: Env, address: Address) {
        env.storage().instance().set(&MockGuardKey::Maintainer(address), &true);
    }

    pub fn remove_maintainer(env: Env, address: Address) {
        env.storage().instance().set(&MockGuardKey::Maintainer(address), &false);
    }
}

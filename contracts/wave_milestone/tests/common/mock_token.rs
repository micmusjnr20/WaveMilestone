use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone)]
pub enum MockTokenKey {
    Balance(Address),
    Admin,
}

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn init(env: Env, admin: Address) {
        env.storage().instance().set(&MockTokenKey::Admin, &admin);
    }

    pub fn mint(env: Env, to: Address, amount: u128) {
        let bal = env
            .storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(to), &(bal + amount));
    }

    pub fn transfer(env: Env, from: Address, to: Address, amount: u128) {
        from.require_auth();
        let from_bal = env
            .storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(from.clone()))
            .unwrap_or(0);
        if from_bal < amount {
            panic!("insufficient balance");
        }
        let to_bal = env
            .storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(to.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(from), &(from_bal - amount));
        env.storage()
            .instance()
            .set(&MockTokenKey::Balance(to), &(to_bal + amount));
    }

    pub fn balance(env: Env, id: Address) -> u128 {
        env.storage()
            .instance()
            .get::<_, u128>(&MockTokenKey::Balance(id))
            .unwrap_or(0)
    }
}

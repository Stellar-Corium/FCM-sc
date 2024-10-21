use soroban_sdk::{contracttype, Address, BytesN, Env, String};

const DAY_LEDGER: u32 = 17280;

#[contracttype]
pub struct ReactorState {
    // This is the asset that is going to be minted by this contract.
    // This contract must be the admin of the asset.
    pub fcm: Address,

    // This is the current mineral available to be extracted
    pub current: u64,

    // The amount of zeroes to put in front of the transaction
    pub difficulty: u32,

    // If this is true, mining is dead
    pub is_nuked: bool,

    // This is the first miner, it becomes the owner of the mine
    pub finder: Address,
}

#[contracttype]
pub struct Block {
    pub index: u64,
    pub message: String,
    pub prev_hash: BytesN<32>,
    pub nonce: u64,
    pub miner: Address,

    // The hash is done with index + message + prev_hash + nonce + miner
    pub hash: BytesN<32>,
    pub timestamp: u64,
}

#[contracttype]
pub struct Stake {
    pub owner: Address,
    pub amount: u128,
    pub cools_at: u64,
}

#[contracttype]
pub enum StorageKeys {
    MineState,
    Block(u64),
    Stake(Address),
}

pub fn pump_core(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(DAY_LEDGER, DAY_LEDGER * 3);
}

pub fn set_state(e: &Env, state: &ReactorState) {
    e.storage().instance().set(&StorageKeys::MineState, state);
}

pub fn get_state(e: &Env) -> Option<ReactorState> {
    e.storage().instance().get(&StorageKeys::MineState)
}

pub fn set_block(e: &Env, attempt: &Block) {
    e.storage()
        .persistent()
        .set(&StorageKeys::Block(attempt.index.clone()), attempt);
}

pub fn get_block(e: &Env, index: &u64) -> Option<Block> {
    e.storage()
        .persistent()
        .get(&StorageKeys::Block(index.clone()))
}

pub fn pump_block(e: &Env, index: &u64) {
    e.storage().persistent().extend_ttl(
        &StorageKeys::Block(index.clone()),
        DAY_LEDGER * 15,
        DAY_LEDGER * 30,
    );
}

pub fn get_stake(e: &Env, miner: &Address) -> Option<Stake> {
    e.storage()
        .persistent()
        .get(&StorageKeys::Stake(miner.clone()))
}

pub fn set_stake(e: &Env, stake: &Stake) {
    e.storage()
        .persistent()
        .set(&StorageKeys::Stake(stake.owner.clone()), stake);
}

pub fn delete_stake(e: &Env, miner: &Address) {
    e.storage()
        .persistent()
        .remove(&StorageKeys::Stake(miner.clone()));
}

pub fn pump_stake(e: &Env, miner: &Address) {
    e.storage().persistent().extend_ttl(
        &StorageKeys::Stake(miner.clone()),
        DAY_LEDGER * 15,
        DAY_LEDGER * 30,
    );
}

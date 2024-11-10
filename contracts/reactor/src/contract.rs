use core::ops::Add;
use hex::encode;

use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Bytes, BytesN, Env, String,
};

use crate::errors::ContractErrors;
use crate::storage::{
    delete_stake, get_attempt, get_block, get_miner_attempt, get_miner_attempt_index, get_stake,
    get_state, pump_block, pump_core, pump_stake, set_attempt, set_block, set_miner_attempt,
    set_miner_attempt_index, set_stake, set_state, Attempt, Block, MinerAttempt, ReactorState,
    Stake,
};

pub const MAX_SUPPLY: u64 = 16_000_000u64;
pub const STAKING_DIVISOR: u64 = 10_000u64;

pub trait ReactorContractTrait {
    fn upgrade(e: Env, hash: BytesN<32>);

    fn set_difficulty(e: &Env, difficulty: u32);

    fn find(e: Env, fcm: Address, miner: Address, message: String);

    fn mine(e: Env, hash: BytesN<32>, message: String, nonce: u64, miner: Address);

    fn stake(e: Env, caller: Address, amount: u128);

    fn un_stake(e: Env, caller: Address);

    fn fkin_nuke_it(e: Env, caller: Address);
}

#[contract]
pub struct ReactorContract;

#[contractimpl]
impl ReactorContractTrait for ReactorContract {
    fn upgrade(e: Env, hash: BytesN<32>) {
        get_state(&e).unwrap().finder.require_auth();
        e.deployer().update_current_contract_wasm(hash);
    }

    fn set_difficulty(e: &Env, difficulty: u32) {
        let mut state = get_state(&e).unwrap();
        state.finder.require_auth();
        state.difficulty = difficulty;
        set_state(&e, &state);
        pump_core(&e);
    }

    fn find(e: Env, fcm: Address, miner: Address, message: String) {
        miner.require_auth();

        if get_state(&e).is_some() {
            panic_with_error!(&e, &ContractErrors::AlreadyDiscovered);
        }

        let prev_hash: BytesN<32> = BytesN::from_array(
            &e,
            &[
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            ],
        );
        let nonce: u64 = 0;
        let hash: BytesN<32> = create_block_hash(&e, &0, &message, &prev_hash, &nonce, &miner);

        let new_attempt: Block = Block {
            index: 0,
            message,
            prev_hash,
            nonce,
            miner: miner.clone(),
            hash,
            timestamp: e.ledger().timestamp(),
        };

        set_block(&e, &new_attempt);
        pump_block(&e, &new_attempt.index);

        set_state(
            &e,
            &ReactorState {
                fcm,
                current: 0,
                difficulty: 0,
                is_nuked: false,
                finder: miner,
            },
        );

        pump_core(&e);
    }

    fn mine(e: Env, hash: BytesN<32>, message: String, nonce: u64, miner: Address) {
        miner.require_auth();

        if message.len() > 64 {
            panic_with_error!(&e, &ContractErrors::MessageIsTooLarge);
        }

        let mut state: ReactorState = get_state(&e).unwrap_or_else(|| {
            panic_with_error!(&e, &ContractErrors::NonDiscovered);
        });

        if state.is_nuked {
            panic_with_error!(&e, &ContractErrors::TheMineWasNuked);
        }

        if state.current >= MAX_SUPPLY {
            panic_with_error!(&e, &ContractErrors::NoMoreSupplyAvailable);
        }

        let new_index: u64 = state.current + 1;
        let prev_attempt: Block = get_block(&e, &state.current).unwrap();

        let generated_hash: BytesN<32> =
            create_block_hash(&e, &new_index, &message, &prev_attempt.hash, &nonce, &miner);

        if hash != generated_hash {
            panic_with_error!(&e, &ContractErrors::ProvidedHashIsInvalid);
        }

        let stake: Stake = get_stake(&e, &miner).unwrap_or(Stake {
            owner: miner.clone(),
            amount: 0,
            cools_at: 0,
        });

        if (stake.amount / 1_0000000) < (state.current / STAKING_DIVISOR) as u128 {
            panic_with_error!(&e, &ContractErrors::NotEnoughStaked);
        }

        if !is_difficulty_correct(&generated_hash, &state.difficulty) {
            panic_with_error!(&e, &ContractErrors::ProvidedDifficultyIsInvalid);
        }

        // We try to save the miner attempt, if there is no more space we just ignore it
        let mut attempt: Attempt = get_attempt(&e, &(state.current + 1)).unwrap_or(Attempt {
            block: state.current + 1,
            total_miners: 0,
        });
        if attempt.total_miners < 255 {
            if get_miner_attempt(&e, &attempt.block, &miner).is_none() {
                attempt.total_miners += 1;
                set_attempt(&e, &attempt.block, &attempt);
                let miner_attempt = MinerAttempt {
                    block: attempt.block,
                    miner: miner.clone(),
                    position: attempt.total_miners,
                };
                set_miner_attempt_index(&e, &miner_attempt);
                set_miner_attempt(&e, &miner_attempt);
            };
        }

        // Mutation is not enabled at the moment
        // mutate_stake_position(&e, &state, &miner);

        let current_block: Block = get_block(&e, &state.current).unwrap();

        // If there's been 60 seconds since the last block, the block is generated
        if e.ledger().timestamp() > (current_block.timestamp + 60) {
            let winner_number: u32 = find_winner(&e, &attempt);
            let winner_miner: Address =
                get_miner_attempt_index(&e, &attempt.block, &winner_number).unwrap();

            let new_attempt: Block = Block {
                index: new_index,
                message,
                prev_hash: prev_attempt.hash,
                nonce,
                timestamp: e.ledger().timestamp(),
                miner: winner_miner,
                hash: generated_hash,
            };

            set_block(&e, &new_attempt);
            pump_block(&e, &new_attempt.index);

            // The protocol tries to send the last found amount based on time to find the block
            match get_block(&e, &(prev_attempt.index.saturating_sub(1))) {
                None => {
                    let _ = token::StellarAssetClient::new(&e, &state.fcm)
                        .try_mint(&prev_attempt.miner, &1_0000000);
                }
                Some(block_before) => {
                    let seconds_to_find: u64 = prev_attempt
                        .timestamp
                        .saturating_sub(block_before.timestamp)
                        .add(1);
                    let mut amount_to_send: i128 = (seconds_to_find / 60) as i128 * 1_0000000i128;
                    if amount_to_send == 0 {
                        amount_to_send += 1_0000000i128;
                    }
                    let _ = token::StellarAssetClient::new(&e, &state.fcm)
                        .try_mint(&prev_attempt.miner, &amount_to_send);
                }
            }

            // We update the index to the new attempt
            state.current = new_index;
        }

        set_state(&e, &state);
        pump_core(&e);
    }

    fn stake(e: Env, miner: Address, amount: u128) {
        miner.require_auth();

        let state: ReactorState = get_state(&e).unwrap_or_else(|| {
            panic_with_error!(&e, &ContractErrors::NonDiscovered);
        });

        token::Client::new(&e, &state.fcm).transfer(
            &miner,
            &e.current_contract_address(),
            &(amount as i128),
        );

        let mut stake: Stake = get_stake(&e, &miner).unwrap_or(Stake {
            owner: miner.clone(),
            amount: 0,
            cools_at: 0,
        });

        stake.cools_at = e.ledger().timestamp() + (3600 * 24 * 60);
        stake.amount += amount;
        set_stake(&e, &stake);

        pump_stake(&e, &miner);
        pump_core(&e);
    }

    fn un_stake(e: Env, miner: Address) {
        miner.require_auth();

        let state: ReactorState = get_state(&e).unwrap_or_else(|| {
            panic_with_error!(&e, &ContractErrors::NonDiscovered);
        });

        let stake: Stake = get_stake(&e, &miner).unwrap_or(Stake {
            owner: miner.clone(),
            amount: 0,
            cools_at: 0,
        });

        if stake.amount == 0 {
            panic_with_error!(&e, &ContractErrors::NothingToWithdraw);
        }

        if stake.cools_at >= e.ledger().timestamp() {
            panic_with_error!(&e, &ContractErrors::StakeIsStillHot);
        }

        token::Client::new(&e, &state.fcm).transfer(
            &e.current_contract_address(),
            &miner,
            &(stake.amount as i128),
        );

        delete_stake(&e, &miner);
        pump_core(&e);
    }

    fn fkin_nuke_it(e: Env, caller: Address) {
        caller.require_auth();

        let mut state: ReactorState = get_state(&e).unwrap_or_else(|| {
            panic_with_error!(&e, &ContractErrors::NonDiscovered);
        });

        if state.is_nuked {
            panic_with_error!(&e, &ContractErrors::TheMineWasNuked);
        }

        if caller != state.finder {
            panic_with_error!(&e, &ContractErrors::NotTheFinder);
        }

        state.is_nuked = true;
        set_state(&e, &state);
    }
}

pub fn create_block_hash(
    e: &Env,
    index: &u64,
    message: &String,
    prev_hash: &BytesN<32>,
    nonce: &u64,
    miner: &Address,
) -> BytesN<32> {
    let mut builder: Bytes = Bytes::new(&e);
    builder.append(&index.to_xdr(&e));
    builder.append(&message.clone().to_xdr(&e));
    builder.append(&prev_hash.clone().to_xdr(&e));
    builder.append(&nonce.to_xdr(&e));
    builder.append(&miner.to_xdr(&e));

    e.crypto().keccak256(&builder).to_bytes()
}

pub fn is_difficulty_correct(hash: &BytesN<32>, difficulty: &u32) -> bool {
    let hex = encode(hash.to_array());
    let mut total_zeroes: u32 = 0;

    for char in hex.chars() {
        if char as u32 != 48 {
            break;
        } else {
            total_zeroes += 1;
        }
    }

    &total_zeroes == difficulty
}

// We could use prng to generate the randomness, but for now will go with this method
// Can be changed later if we find an issue with it
pub fn find_winner(e: &Env, attempt: &Attempt) -> u32 {
    let mut builder: Bytes = Bytes::new(&e);
    builder.append(&attempt.block.to_xdr(&e));
    builder.append(&e.ledger().sequence().to_xdr(&e));
    builder.append(&e.ledger().timestamp().to_xdr(&e));
    builder.append(&attempt.total_miners.to_xdr(&e));
    let hash: BytesN<32> = e.crypto().keccak256(&builder).to_bytes();
    (hash.get(16).unwrap() as u32 % (attempt.total_miners)) + 1
}

pub fn mutate_stake_position(e: &Env, state: &ReactorState, address: &Address) {
    let mut stake: Stake = get_stake(&e, &address).unwrap_or(Stake {
        owner: address.clone(),
        amount: 0,
        cools_at: 0,
    });

    if stake.amount > 0 {
        if e.prng().gen_range::<u64>(0..=100) < 33 {
            token::Client::new(&e, &state.fcm)
                .burn(&e.current_contract_address(), &(stake.amount as i128));
            delete_stake(&e, &stake.owner);
        } else {
            pump_stake(&e, &stake.owner);
            token::StellarAssetClient::new(&e, &state.fcm)
                .mint(&e.current_contract_address(), &(stake.amount as i128));
            stake.amount += stake.amount;
            set_stake(&e, &stake);
        }
    }
}

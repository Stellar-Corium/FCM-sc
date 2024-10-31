#![cfg(test)]

use crate::contract::create_block_hash;
use crate::errors::ContractErrors;
use crate::storage::{get_block, get_state, set_state, Block, ReactorState};
use crate::tests::test_utils::{create_test_data, start_contract, TestData};
use soroban_sdk::testutils::{Address as _, BytesN as __, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal, String};

#[test]
fn test_mining_a_block() {
    let e: Env = Env::default();
    e.ledger().set_timestamp(60);

    let test_data: TestData = create_test_data(&e);
    start_contract(&e, &test_data);

    let miner: Address = Address::generate(&e);
    let message: String = String::from_str(&e, "Second block");

    let mut prev_block_option: Option<Block> = None;
    e.as_contract(&test_data.contract_client.address, || {
        prev_block_option = Some(get_block(&e, &0).unwrap());
    });

    let prev_block: Block = prev_block_option.unwrap();

    let hash: BytesN<32> = create_block_hash(&e, &1, &message, &prev_block.hash, &0, &miner);

    assert!(test_data
        .contract_client
        .try_mine(&hash, &message, &0, &miner)
        .is_err());

    assert_eq!(
        test_data.fcm_client.balance(&test_data.genesis_block_miner),
        0
    );

    e.ledger().set_timestamp(3660);

    test_data
        .contract_client
        .mock_auths(&[MockAuth {
            address: &miner,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract_client.address,
                fn_name: "mine",
                args: (hash.clone(), message.clone(), 0u64, miner.clone()).into_val(&e),
                sub_invokes: &[],
            },
        }])
        .mine(&hash, &message, &0, &miner);

    e.as_contract(&test_data.contract_client.address, || {
        let state: ReactorState = get_state(&e).unwrap();
        assert_eq!(state.current, 1);
        assert_eq!(state.difficulty, 0);
    });

    assert_eq!(
        test_data.fcm_client.balance(&test_data.genesis_block_miner),
        1_0000000
    );

    let mut new_prev_block_option: Option<Block> = None;
    e.as_contract(&test_data.contract_client.address, || {
        new_prev_block_option = Some(get_block(&e, &1).unwrap());
    });
    let new_prev_block: Block = new_prev_block_option.unwrap();

    let new_hash: BytesN<32> =
        create_block_hash(&e, &2, &message, &new_prev_block.hash, &0, &miner);

    e.ledger().set_timestamp(190);

    test_data
        .contract_client
        .mock_all_auths()
        .mine(&new_hash, &message, &0, &miner);

    // Because it took an hour to find the block, it should send 61 FCMs
    assert_eq!(test_data.fcm_client.balance(&miner), 61_0000000i128);
}

#[test]
fn test_mining_errors() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);

    let miner: Address = Address::generate(&e);
    let too_large_message: String = String::from_str(&e, "Cras luctus gravida ornare. Integer rhoncus eros eu gravida congue. Nam egestas facilisis erat vitae volutpat. Pellentesque et purus facilisis, porttitor libero vitae.");
    let nonce: u64 = 0;

    let too_large_message_error = test_data
        .contract_client
        .mock_all_auths()
        .try_mine(&BytesN::random(&e), &too_large_message, &nonce, &miner)
        .unwrap_err()
        .unwrap();

    assert_eq!(
        too_large_message_error,
        ContractErrors::MessageIsTooLarge.into()
    );

    let message: String = String::from_str(&e, "random message");

    let non_discovered_error = test_data
        .contract_client
        .mock_all_auths()
        .try_mine(&BytesN::random(&e), &message, &nonce, &miner)
        .unwrap_err()
        .unwrap();

    assert_eq!(non_discovered_error, ContractErrors::NonDiscovered.into());

    start_contract(&e, &test_data);

    let mut prev_block_option: Option<Block> = None;
    e.as_contract(&test_data.contract_client.address, || {
        prev_block_option = Some(get_block(&e, &0).unwrap());
    });
    let prev_block: Block = prev_block_option.unwrap();
    let hash: BytesN<32> = create_block_hash(&e, &1, &message, &prev_block.hash, &nonce, &miner);

    let invalid_provided_hash_error = test_data
        .contract_client
        .mock_all_auths()
        .try_mine(&hash, &message, &(nonce + 1), &miner)
        .unwrap_err()
        .unwrap();

    assert_eq!(
        invalid_provided_hash_error,
        ContractErrors::ProvidedHashIsInvalid.into()
    );

    e.as_contract(&test_data.contract_client.address, || {
        let mut state: ReactorState = get_state(&e).unwrap();
        state.difficulty = 10;
        set_state(&e, &state);
    });

    let invalid_provided_difficulty_error = test_data
        .contract_client
        .mock_all_auths()
        .try_mine(&hash, &message, &nonce, &miner)
        .unwrap_err()
        .unwrap();

    assert_eq!(
        invalid_provided_difficulty_error,
        ContractErrors::ProvidedDifficultyIsInvalid.into()
    );

    e.as_contract(&test_data.contract_client.address, || {
        let mut state: ReactorState = get_state(&e).unwrap();
        state.difficulty = 0;
        set_state(&e, &state);
    });
}

#[test]
fn test_mining_with_high_difficulty() {
    let e: Env = Env::default();
    e.mock_all_auths();
    e.budget().reset_unlimited();
    let test_data: TestData = create_test_data(&e);
    start_contract(&e, &test_data);

    let difficulty: u32 = 6;

    e.as_contract(&test_data.contract_client.address, || {
        let mut state = get_state(&e).unwrap();
        state.difficulty = difficulty;
        set_state(&e, &state);
    });

    let miner: Address = Address::from_string(&String::from_str(
        &e,
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAITA4",
    ));
    let message: String = String::from_str(&e, "The random message");
    let mut prev_block_option: Option<Block> = None;
    e.as_contract(&test_data.contract_client.address, || {
        prev_block_option = Some(get_block(&e, &0).unwrap());
    });
    let prev_block: Block = prev_block_option.unwrap();

    let nonce: u64 = 5114425;
    let hash: BytesN<32> = create_block_hash(&e, &1, &message, &prev_block.hash, &nonce, &miner);

    test_data
        .contract_client
        .mock_all_auths()
        .mine(&hash, &message, &nonce, &miner);

    // Hash: BytesN<32>(0, 0, 0, 56, 38, 109, 57, 170, 142, 27, 143, 23, 149, 96, 45, 107, 234, 142, 67, 208, 5, 191, 37, 66, 121, 193, 142, 94, 226, 70, 117, 5)
    // Hash in Hex format: "00000038266d39aa8e1b8f1795602d6bea8e43d005bf254279c18e5ee2467505"
}

// TODO: Test variable mining, test staking requirements, etc etc etc

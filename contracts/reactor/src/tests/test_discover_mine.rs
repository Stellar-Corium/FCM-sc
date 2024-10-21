#![cfg(test)]

use soroban_sdk::testutils::{Address as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal, String};

use crate::contract::create_block_hash;
use crate::errors::ContractErrors;
use crate::storage::{get_block, get_state, Block, ReactorState};
use crate::tests::test_utils::{create_test_data, TestData};

#[test]
fn test_find() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    let genesis_block_miner: Address = Address::generate(&e);
    let message: String = String::from_str(&e, "Hello World!");

    assert!(test_data
        .contract_client
        .try_find(
            &test_data.fcm_client.address,
            &genesis_block_miner,
            &message,
        )
        .is_err());

    test_data
        .contract_client
        .mock_auths(&[MockAuth {
            address: &genesis_block_miner,
            invoke: &MockAuthInvoke {
                contract: &test_data.contract_client.address,
                fn_name: "find",
                args: (
                    test_data.fcm_client.address.clone(),
                    genesis_block_miner.clone(),
                    message.clone(),
                )
                    .into_val(&e),
                sub_invokes: &[],
            },
        }])
        .find(
            &test_data.fcm_client.address,
            &genesis_block_miner,
            &message,
        );

    e.as_contract(&test_data.contract_client.address, || {
        let state: ReactorState = get_state(&e).unwrap();
        assert_eq!(state.current, 0);
        assert_eq!(state.difficulty, 0);
        assert_eq!(state.fcm, test_data.fcm_client.address);

        let genesis_block: Block = get_block(&e, &0).unwrap();
        assert_eq!(genesis_block.timestamp, 0);
        assert_eq!(genesis_block.index, 0);
        assert_eq!(genesis_block.miner, genesis_block_miner);
        assert_eq!(
            genesis_block.prev_hash,
            BytesN::from_array(
                &e,
                &[
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                    0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                ],
            )
        );
        assert_eq!(genesis_block.nonce, 0);
        assert_eq!(genesis_block.message, message);
        assert_eq!(
            genesis_block.hash,
            create_block_hash(
                &e,
                &0,
                &message,
                &BytesN::from_array(
                    &e,
                    &[
                        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                        0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
                        0u8, 0u8,
                    ],
                ),
                &0,
                &genesis_block_miner
            )
        );
    });
}

#[test]
fn test_already_discovered_error() {
    let e: Env = Env::default();
    let test_data: TestData = create_test_data(&e);
    let genesis_block_miner: Address = Address::generate(&e);
    let message: String = String::from_str(&e, "Hello World!");

    test_data.contract_client.mock_all_auths().find(
        &test_data.fcm_client.address,
        &genesis_block_miner,
        &message,
    );

    let error = test_data
        .contract_client
        .mock_all_auths()
        .try_find(
            &test_data.fcm_client.address,
            &genesis_block_miner,
            &message,
        )
        .unwrap_err()
        .unwrap();

    assert_eq!(error, ContractErrors::AlreadyDiscovered.into());
}

#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env, String};

use crate::contract::{ReactorContract, ReactorContractClient};

pub struct TestData<'a> {
    pub contract_client: ReactorContractClient<'a>,
    pub fcm_client: token::Client<'a>,
    pub genesis_block_miner: Address,
}

pub fn create_test_data<'a>(e: &Env) -> TestData<'a> {
    let contract_id: Address = e.register_contract(None, ReactorContract);
    let contract_client: ReactorContractClient<'a> = ReactorContractClient::new(&e, &contract_id);

    let contract_address = e.register_stellar_asset_contract_v2(contract_client.address.clone());
    let fcm_client = token::Client::new(e, &contract_address.address());
    let genesis_block_miner: Address = Address::generate(&e);

    TestData {
        contract_client,
        fcm_client,
        genesis_block_miner,
    }
}

pub fn start_contract(e: &Env, test_data: &TestData) {
    let message: String = String::from_str(&e, "Hello World!");
    test_data.contract_client.mock_all_auths().find(
        &test_data.fcm_client.address,
        &test_data.genesis_block_miner,
        &message,
    );
}

use std::thread;

use eyre::Result;
use iroha_client::{client, data_model::prelude::*};
use iroha_config::parameters::actual::Root as Config;
use test_network::*;

use crate::integration::new_account_with_random_public_key;

#[test]
// This test suite is also covered at the UI level in the iroha_client_cli tests
// in test_register_accounts.py
fn client_add_account_with_name_length_more_than_limit_should_not_commit_transaction() -> Result<()>
{
    let (_rt, _peer, test_client) = <PeerBuilder>::new().with_port(10_505).start_with_runtime();
    wait_for_genesis_committed(&vec![test_client.clone()], 0);

    let pipeline_time = Config::pipeline_time();

    let normal_account_id: AccountId = "bob@wonderland".parse().expect("Valid");
    let create_account = Register::account(new_account_with_random_public_key(
        normal_account_id.clone(),
    ));
    test_client.submit(create_account)?;

    let too_long_account_name = "0".repeat(2_usize.pow(14));
    let incorrect_account_id: AccountId = (too_long_account_name + "@wonderland")
        .parse()
        .expect("Valid");
    let create_account = Register::account(new_account_with_random_public_key(
        incorrect_account_id.clone(),
    ));
    test_client.submit(create_account)?;

    thread::sleep(pipeline_time * 2);

    assert!(test_client
        .request(client::account::by_id(normal_account_id))
        .is_ok());
    assert!(test_client
        .request(client::account::by_id(incorrect_account_id))
        .is_err());

    Ok(())
}

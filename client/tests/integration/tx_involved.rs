use iroha_client::{client::transaction, crypto::KeyPair, data_model::prelude::*};
use std::str::FromStr;
use test_network::*;

#[test]
fn get_involved_txs_quantity() {
    get_involved_txs(10_710)
}

// TODO add tests when the transfer uses the wrong AssetId.

fn get_involved_txs(port_number: u16) {
    let (_rt, _peer, iroha_client) = <PeerBuilder>::new()
        .with_port(port_number)
        .start_with_runtime();
    wait_for_genesis_committed(&[iroha_client.clone()], 0);

    let alice_id: AccountId = "alice@wonderland".parse().expect("Valid");
    let mouse_id: AccountId = "mouse@wonderland".parse().expect("Valid");
    let (bob_public_key, _) = KeyPair::generate()
        .expect("Failed to generate KeyPair")
        .into();
    let create_mouse = RegisterExpr::new(Account::new(mouse_id.clone(), [bob_public_key]));
    let asset_definition_id: AssetDefinitionId = "camomile#wonderland".parse().expect("Valid");
    let create_asset = RegisterExpr::new(AssetDefinition::quantity(asset_definition_id.clone()));
    let mint_asset = MintExpr::new(
        1_u32.to_value(),
        IdBox::AssetId(AssetId::new(asset_definition_id.clone(), alice_id.clone())),
    );

    let instructions: [InstructionExpr; 3] = [
        // create_alice.into(), We don't need to register Alice, because she is created in genesis
        create_mouse.into(),
        create_asset.into(),
        mint_asset.into(),
    ];
    iroha_client
        .submit_all_blocking(instructions)
        .expect("Failed to prepare state.");

    let mut metadata = UnlimitedMetadata::new();
    metadata.insert(
        Name::from_str("involved_accounts").unwrap(),
        Value::Vec(vec![mouse_id.clone().to_value()]),
    );
    {
        let transfer_asset = TransferExpr::new(
            IdBox::AssetId(AssetId::new(asset_definition_id.clone(), alice_id)),
            1_u32.to_value(),
            IdBox::AccountId(mouse_id.clone()),
        );
        iroha_client
            .submit_all_blocking(vec![transfer_asset])
            .expect("Failed to prepare state.");
    }
    {
        let mint_asset = MintExpr::new(
            1_u32.to_value(),
            IdBox::AssetId(AssetId::new(asset_definition_id.clone(), mouse_id.clone())),
        );
        iroha_client
            .submit_all_blocking(vec![mint_asset])
            .expect("Failed to prepare state.");
    }
    {
        let burn_asset = BurnExpr::new(
            1_u32.to_value(),
            IdBox::AssetId(AssetId::new(asset_definition_id.clone(), mouse_id.clone())),
        );
        let tx = iroha_client
            .build_transaction(vec![burn_asset], metadata.clone())
            .expect("Valid");
        let signed_tx = iroha_client.sign_transaction(tx).expect("Valid");
        iroha_client
            .submit_transaction_blocking(&signed_tx)
            .expect("Valid");
    }

    let response = iroha_client
        .request(transaction::by_account_id_involved(mouse_id.clone()))
        .unwrap();

    let mut involved_tx_count = 0;
    for tx in response {
        let tx = tx.expect("Transaction should be Ok");
        let transaction_value = tx.transaction();
        assert!(
            transaction_value.error.is_none(),
            "Transaction should not have errors"
        );
        involved_tx_count += 1;

        println!("Transaction error: {:?}", transaction_value.error);
    }

    // 1. register mouse
    // 2. transfer asset to mouse from alice
    // 3. mint asset for mouse
    // 4. burn with involved_accounts metadata
    assert_eq!(involved_tx_count, 4);
}

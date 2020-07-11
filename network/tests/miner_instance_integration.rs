mod miner_instance_integration {
    use snarkos_dpc::base_dpc::instantiated::{CommitmentMerkleParameters, Tx};
    use snarkos_network::server::MinerInstance;
    use snarkos_testing::{consensus::*, dpc::load_verifying_parameters, network::*, storage::kill_storage_async};

    use serial_test::serial;
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    #[test]
    #[serial]
    fn spawn_and_mine() {
        let mut rt = Runtime::new().unwrap();
        let storage = Arc::new(FIXTURE_VK.ledger());
        let path = storage.storage.db.path().to_owned();
        let parameters = load_verifying_parameters();

        rt.block_on(async move {
            let bootnode_address = random_socket_address();
            let server_address = random_socket_address();

            // 1. Get server details

            let server = initialize_test_server(
                server_address,
                bootnode_address,
                storage,
                parameters,
                CONNECTION_FREQUENCY_LONG,
            );

            // 2. Create miner instance

            let miner_address = FIXTURE_VK.test_accounts[0].address.clone();

            let miner = MinerInstance::new(
                miner_address,
                server.consensus.clone(),
                server.parameters.clone(),
                server.storage.clone(),
                server.memory_pool_lock.clone(),
                server.context.clone(),
            );

            // 3. Spawn miner

            miner.spawn();
        });

        // Kill the miner
        drop(rt);
        kill_storage_async::<Tx, CommitmentMerkleParameters>(path);
    }
}

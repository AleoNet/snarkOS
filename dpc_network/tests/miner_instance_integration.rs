// mod miner_instance_integration {
//     use snarkos_consensus::test_data::*;
//     use snarkos_network::{server::MinerInstance, test_data::*};
//
//     use serial_test::serial;
//     use std::str::FromStr;
//     use tokio::runtime;
//     use wagyu_bitcoin::{BitcoinAddress, Mainnet};
//
//     type N = Mainnet;
//
//     #[test]
//     #[serial]
//     fn spawn_and_mine() {
//         let (storage, path) = initialize_test_blockchain();
//
//         let mut rt = runtime::Runtime::new().unwrap();
//
//         rt.block_on(async move {
//             let bootnode_address = random_socket_address();
//             let server_address = aleo_socket_address();
//
//             // 1. Get server details
//
//             let server = initialize_test_server(server_address, bootnode_address, storage, CONNECTION_FREQUENCY_LONG);
//
//             // 2. Create miner instance
//
//             let miner = MinerInstance::new(
//                 BitcoinAddress::<N>::from_str(TEST_WALLETS[4].address).unwrap(),
//                 server.consensus.clone(),
//                 server.storage.clone(),
//                 server.memory_pool_lock.clone(),
//                 server.context.clone(),
//             );
//
//             // 3. Spawn miner
//
//             miner.spawn();
//         });
//
//         // Kill the miner
//         drop(rt);
//         kill_storage_async(path);
//     }
// }

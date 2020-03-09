use crate::ConsensusParameters;
use snarkos_storage::BlockStorage;

use std::{path::PathBuf, str::FromStr, sync::Arc};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

pub const TEST_DB_PATH: &str = "../test_db";

pub const TEST_CONSENSUS: ConsensusParameters = ConsensusParameters {
    max_block_size: 1_000_000usize, // coinbase + 1 transaction
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
    transaction_size: 366usize,
};

pub const TRANSACTION: &str = "0100000001b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee355000000006a473045022100a3a47eb43eed300927ac841483eaae4f7a886f9fcd7316dae83c9e73d0b4b11802206e948f63dccd11d01cd59b8bbee0251bc16122e3e7b1559f34a3f958534e45342103ca64499d857698431e999035fd22d97896b1dff672739ad9acb8643cdd2be95102f8dcfa02000000001976a9143804a328df69bc873f96c63b3e3218bc2602283088acf8dcfa02000000001976a9148e3d6baa7c1a0a927ea69108503fb5b55e9a71eb88ac";

pub const STANDARD_TX_FEE: u64 = 10000;

pub struct Wallet {
    pub private_key: &'static str,
    pub address: &'static str,
}

pub const TEST_WALLETS: [Wallet; 5] = [
    Wallet {
        private_key: "KzW6KyJ1s4mp3CFDUzCXFh4r2xzyd2rkMwcbeP5T2T2iMvepkAwS",
        address: "1NpScgYSLW4WcvmZM55EY5cziEiqZx3wJu",
    },
    Wallet {
        private_key: "L2tBggaVMYPghRB6LR2ThY5Er1Rc284T3vgiK274JpaFsj1tVSsT",
        address: "167CPx9Ae96iVQCrwoq17jwKmmvr9RTyM7",
    },
    Wallet {
        private_key: "KwrJGqYZVj3m2WyimxdLBNrdwQZBVnHhw78c73xuLSWkjFBiqq3P",
        address: "1Dy6XpKrNRDw9SewppvYpGHSMbBExVmZsU",
    },
    Wallet {
        private_key: "KwwZ97gYoBBf6cGLp33qD8v4pEKj89Yir65vUA3N5Y1AtWbLzqED",
        address: "1CL1zq3kLK3TFNLdTk4HtuguT7JMdD5vi5",
    },
    Wallet {
        private_key: "L4cR7BQfvj6CPdbaTvRKHJXB4LjaUHJxtrDqNzkkyRXqrqUxLQTS",
        address: "1Hz8RzEXYPF6z8o7z5SHVnjzmhqS5At5kU",
    },
];

pub const GENESIS_BLOCK: &str = "0000000000000000000000000000000000000000000000000000000000000000b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee35592ec145e00000000ffffffffff7f000030d901000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04080000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";
pub const GENESIS_BLOCK_LATE: &str = "0000000000000000000000000000000000000000000000000000000000000000b4833ddb61c76a98e02d9ffc88e46f20aec673a201323cf0a7311be12a2d91a2f4ff215e00000000ffffffffff7f00002fa604000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04010000000100e1f505000000001976a91447862fe165e6121af80d5dde1ecb478ed170565b88ac";

pub const GENESIS_TRANSACTION: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04080000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";
pub const GENESIS_TRANSACTION_ID: &str = "b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee355";
pub const GENESIS_BLOCK_HEADER_HASH: &str = "3a8a5db71a2e00007b47cac0c43e5b96ca6f0107dd98ab568ac51b829856a46a";
pub const GENESIS_BLOCK_GENESIS_MINER_BALANCE: u64 = 100_000_000;

pub const BLOCK_1: &str = "3a8a5db71a2e00007b47cac0c43e5b96ca6f0107dd98ab568ac51b829856a46a7cf53adb19d9892a93bbf05622c37fc881f8ff61bb01cd8fba16c177abc2c13a751b1d5e00000000feffffffffff00000aa603000201000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0401000000011008f605000000001976a914ba4fecdfa1d8a56dbf248f1337cefdf06cfc1f6a88ac0100000001b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee355000000006a473045022100a3a47eb43eed300927ac841483eaae4f7a886f9fcd7316dae83c9e73d0b4b11802206e948f63dccd11d01cd59b8bbee0251bc16122e3e7b1559f34a3f958534e45342103ca64499d857698431e999035fd22d97896b1dff672739ad9acb8643cdd2be95102f8dcfa02000000001976a9143804a328df69bc873f96c63b3e3218bc2602283088acf8dcfa02000000001976a9148e3d6baa7c1a0a927ea69108503fb5b55e9a71eb88ac";
pub const BLOCK_1_LATE: &str = "3a8a5db71a2e00007b47cac0c43e5b96ca6f0107dd98ab568ac51b829856a46a76329ff15f33d89cf27852d754cfd6665f979a4edef422a318b7e15163e8438f690b225e00000000feffffffffff000004e400000201000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0402000000011008f605000000001976a914ba4fecdfa1d8a56dbf248f1337cefdf06cfc1f6a88ac0100000001b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee355000000006a473045022100a3a47eb43eed300927ac841483eaae4f7a886f9fcd7316dae83c9e73d0b4b11802206e948f63dccd11d01cd59b8bbee0251bc16122e3e7b1559f34a3f958534e45342103ca64499d857698431e999035fd22d97896b1dff672739ad9acb8643cdd2be95102f8dcfa02000000001976a9143804a328df69bc873f96c63b3e3218bc2602283088acf8dcfa02000000001976a9148e3d6baa7c1a0a927ea69108503fb5b55e9a71eb88ac";

pub const BLOCK_1_HEADER_HASH: &str = "c45bdae15f7300003c224e8e0bbf63c7053f8a68957bd463e682cef1663daa60";
pub const BLOCK_1_TRANSACTION: &str = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04010000000110270000000000001976a914ba4fecdfa1d8a56dbf248f1337cefdf06cfc1f6a88ac";
pub const BLOCK_1_TRANSACTION_ID: &str = "1516e5284b585532327e29e53ee391fa00e8918337c93b62b16c03a4700f9037";
pub const BLOCK_1_GENESIS_MINER_BALANCE: u64 = 0;
pub const BLOCK_1_MINER_BALANCE: u64 = 100_010_000;
pub const BLOCK_1_BALANCE_1: u64 = 49_995_000;
pub const BLOCK_1_BALANCE_2: u64 = 49_995_000;
pub const BLOCK_1_BALANCE_3: u64 = 0;

pub const BLOCK_2: &str = "c45bdae15f7300003c224e8e0bbf63c7053f8a68957bd463e682cef1663daa6054169594958f627235d116990887c71e2c2ca4287fda299de6a04994ba36a312831b1d5e000000006366666666660100920801000301000000010000000000000000000000000000000000000000000000000000000000000000ffffffff040200000001202ff605000000001976a914ba4fecdfa1d8a56dbf248f1337cefdf06cfc1f6a88ac01000000012e88ee424def71c2b8adb495294b633aa17e62cd611678119926acebf3baa41100000000694630440220655046180d503501d4c47351af3c937c04c4c500e8d14dfcb751948a33c990b9022025309e26308ae88f2c103bbd5d994ecfeb1bc91927e0111ab8a60a43b28c94392102d20a50338ac8a524140d1f0e14d0ac1e3c18627acdadd8c7cb139638ff31b94401e8b5fa02000000001976a9147c421e82d4b2a9c77300f8a8c38f42fd30296f4a88ac01000000012e88ee424def71c2b8adb495294b633aa17e62cd611678119926acebf3baa4110100000069463044022075b5e294e1d9dfb232061851a2db3fa36a97729ff043e8d4ed637d6bdb7c33280220223a8cc8541c7a31a9e46e02c7aa517148791f48c33550ed976398be0483caa9210322a35fe703c25153060767011e4d0bdc9adf437b77f124a6167e84b9529e772001e8b5fa02000000001976a9147c421e82d4b2a9c77300f8a8c38f42fd30296f4a88ac";
pub const BLOCK_2_LATE: &str = "c45bdae15f7300003c224e8e0bbf63c7053f8a68957bd463e682cef1663daa6060b3fc0c1091087eb98931a337fd969e5c56d68fb986b0c1ac1bdb5e409359cd66d6385e00000000fcffffffffff0100ca2a305e0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04030000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";

pub const BLOCK_2_GENESIS_MINER_BALANCE: u64 = 0;
pub const BLOCK_2_MINER_BALANCE: u64 = 200_030_000;
pub const BLOCK_2_BALANCE_1: u64 = 0;
pub const BLOCK_2_BALANCE_2: u64 = 0;
pub const BLOCK_2_BALANCE_3: u64 = 99_970_000;

pub const BLOCK_3: &str = "31ce929ea3d70000c08771d05720d0f1c7b4027f555b482825f5329be94b67fd6b688fa8b00199676112f833d65aa9b008d686ae0bbb7158fefc2af10b58b890aed5385e00000000c6cccccccccc02000648a9c90101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04040000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";
pub const BLOCK_3_LATE: &str = "31ce929ea3d70000c08771d05720d0f1c7b4027f555b482825f5329be94b67fd6b688fa8b00199676112f833d65aa9b008d686ae0bbb7158fefc2af10b58b890a2d6385e00000000c6cccccccccc0200c5cd9a8e0101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04040000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";

pub const BLOCK_3_GENESIS_MINER_BALANCE: u64 = 100_000_000;
pub const BLOCK_3_MINER_BALANCE: u64 = 200_030_000;
pub const BLOCK_3_BALANCE_1: u64 = 0;
pub const BLOCK_3_BALANCE_2: u64 = 0;
pub const BLOCK_3_BALANCE_3: u64 = 99_970_000;

pub fn initialize_test_blockchain() -> (Arc<BlockStorage>, PathBuf) {
    let mut path = std::env::current_dir().unwrap();
    path.push(random_storage_path());

    BlockStorage::destroy_storage(path.clone()).unwrap();

    let blockchain = BlockStorage::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

    (blockchain, path)
}

pub fn random_storage_path() -> String {
    let ptr = Box::into_raw(Box::new(123));
    format!("{}{}", TEST_DB_PATH, ptr as usize)
}

pub fn kill_storage_async(path: PathBuf) {
    BlockStorage::destroy_storage(path).unwrap();
}

pub fn kill_storage_sync(storage: Arc<BlockStorage>, path: PathBuf) {
    drop(storage);
    BlockStorage::destroy_storage(path).unwrap();
}

pub fn check_block_1_balances(blockchain: &BlockStorage) {
    let genesis_miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[0].address).unwrap();
    let miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[4].address).unwrap();
    let recipient_1 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[1].address).unwrap();
    let recipient_2 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[2].address).unwrap();
    let recipient_3 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[3].address).unwrap();

    let genesis_miner_balance = blockchain.get_balance(&genesis_miner_address);
    let miner_balance = blockchain.get_balance(&miner_address);
    let balance_1 = blockchain.get_balance(&recipient_1);
    let balance_2 = blockchain.get_balance(&recipient_2);
    let balance_3 = blockchain.get_balance(&recipient_3);

    assert_eq!(genesis_miner_balance, BLOCK_1_GENESIS_MINER_BALANCE);
    assert_eq!(miner_balance, BLOCK_1_MINER_BALANCE);
    assert_eq!(balance_1, BLOCK_1_BALANCE_1);
    assert_eq!(balance_2, BLOCK_1_BALANCE_2);
    assert_eq!(balance_3, BLOCK_1_BALANCE_3);
}

pub fn check_block_2_balances(blockchain: &BlockStorage) {
    let genesis_miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[0].address).unwrap();
    let miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[4].address).unwrap();
    let recipient_1 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[1].address).unwrap();
    let recipient_2 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[2].address).unwrap();
    let recipient_3 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[3].address).unwrap();

    let genesis_miner_balance = blockchain.get_balance(&genesis_miner_address);
    let miner_balance = blockchain.get_balance(&miner_address);
    let balance_1 = blockchain.get_balance(&recipient_1);
    let balance_2 = blockchain.get_balance(&recipient_2);
    let balance_3 = blockchain.get_balance(&recipient_3);

    assert_eq!(genesis_miner_balance, BLOCK_2_GENESIS_MINER_BALANCE);
    assert_eq!(miner_balance, BLOCK_2_MINER_BALANCE);
    assert_eq!(balance_1, BLOCK_2_BALANCE_1);
    assert_eq!(balance_2, BLOCK_2_BALANCE_2);
    assert_eq!(balance_3, BLOCK_2_BALANCE_3);
}

pub fn check_block_3_balances(blockchain: &BlockStorage) {
    let genesis_miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[0].address).unwrap();
    let miner_address = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[4].address).unwrap();
    let recipient_1 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[1].address).unwrap();
    let recipient_2 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[2].address).unwrap();
    let recipient_3 = BitcoinAddress::<Mainnet>::from_str(TEST_WALLETS[3].address).unwrap();

    let genesis_miner_balance = blockchain.get_balance(&genesis_miner_address);
    let miner_balance = blockchain.get_balance(&miner_address);
    let balance_1 = blockchain.get_balance(&recipient_1);
    let balance_2 = blockchain.get_balance(&recipient_2);
    let balance_3 = blockchain.get_balance(&recipient_3);

    assert_eq!(genesis_miner_balance, BLOCK_3_GENESIS_MINER_BALANCE);
    assert_eq!(miner_balance, BLOCK_3_MINER_BALANCE);
    assert_eq!(balance_1, BLOCK_3_BALANCE_1);
    assert_eq!(balance_2, BLOCK_3_BALANCE_2);
    assert_eq!(balance_3, BLOCK_3_BALANCE_3);
}

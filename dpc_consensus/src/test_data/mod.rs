use crate::ConsensusParameters;
use snarkos_dpc::{
    address::{AddressPair, AddressPublicKey},
    base_dpc::{instantiated::*, predicate::DPCPredicate, record::DPCRecord},
    DPCScheme,
};
use snarkos_objects::{
    dpc::{transactions::DPCTransactions, Block},
    ledger::Ledger,
    merkle_root,
    BlockHeader,
    MerkleRootHash,
};

use rand::Rng;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub const TEST_CONSENSUS: ConsensusParameters = ConsensusParameters {
    max_block_size: 1_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 2i64, //unix seconds
};

pub const TEST_DB_PATH: &str = "../test_db";

pub fn random_storage_path() -> String {
    let ptr = Box::into_raw(Box::new(123));
    format!("{}{}", TEST_DB_PATH, ptr as usize)
}

pub fn initialize_test_blockchain() -> (Arc<MerkleTreeLedger>, PathBuf) {
    let mut path = std::env::current_dir().unwrap();
    path.push(random_storage_path());

    MerkleTreeLedger::destroy_storage(path.clone()).unwrap();

    let blockchain = Store::open_at_path(path.clone()).unwrap();

    (blockchain, path)
}

pub struct Wallet {
    pub private_key: &'static str,
    pub address: &'static str,
}

pub const TEST_WALLETS: [Wallet; 3] = [
    Wallet {
        private_key: "49bffc222d6ec7b29254b6d3fe844601141d30cbf6ae3b17efb8a872df9b88128982e8daa5d70d8aefab15a7b6bb052efb7b60f4ca9d88de24dde78641f13102cba821b56f1430fba5173c317f12f67e7162d1b7a3cf5ba6b62f92e4c794d600930b39671c6a1a6d21ff641dd1e1ae431fa889a70d41e3f8ab2a23808ef3dff80101010101010101010101010101010101010101010101010101010101010101c8b3a3d021c55b4b750195eb262ed503597fa073f293ace9048aa12edc897802",
        address: "ff4dcceb9f3003ea59df2770ed4a61409dceb87d244c1be394c52787817ec511",
    },
    Wallet {
        private_key: "d1c3fc5883493d8d79ede69c568dcdcd4050394398910d72ecab098e7763aa057c2f8f271453b47e64e1a9304d71b4beb6f228d76f031ca9328b9d411b260110c452af9bccb1b23865dcdc53b23cc7b31b3a636a55d16c8577c2ec1d0ef1ca02837da510c8b10a673ebb555bfe02314d7e2ece3ed4ac0c08849879af0f826a570202020202020202020202020202020202020202020202020202020202020202e4d5e3868b79fc0b3ba469335f99ce126a56be804660af290575d03d36396804",
        address: "90c0290b0913f0679ae6b27dde990a22863e14bced9125da7f446e5e953af900",
    },
    Wallet {
        private_key: "becf87507e65795452167eba0a9535a96eeb267e673c060d75c34757bca73a112ead6475c492b823c252512e4603f7ed132e45c77c76d97150349bdd7fe001123e49f6dfd7f4d5aea4323832217c4eada6fd2378ca52fea9cbd0a444886ca102a5499008ecf3fe82d977aabfc358669a405887c0d587a3df4a6145b0b99a066d03030303030303030303030303030303030303030303030303030303030303035383e4487adc14bc86504c479e669280488801d631ed1be5e2c4e2eed1fc6104",
        address: "bf774bf47dee3b6ecced6b09f9345c047c51ba3eab035f4fe6cfd4b247fc3e01",
    },
];

pub fn create_block_with_coinbase_transaction<R: Rng>(
    transactions: &mut DPCTransactions<Tx>,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    genesis_pred_vk_bytes: &Vec<u8>,
    new_birth_predicates: Vec<DPCPredicate<Components>>,
    new_death_predicates: Vec<DPCPredicate<Components>>,
    genesis_address: AddressPair<Components>,
    recipient: AddressPublicKey<Components>,
    consensus: &ConsensusParameters,
    ledger: &MerkleTreeLedger,
    rng: &mut R,
) -> (Vec<DPCRecord<Components>>, Block<Tx>) {
    let (new_coinbase_records, transaction) = ConsensusParameters::create_coinbase_transaction(
        ledger.len() as u32,
        &transactions,
        &parameters,
        &genesis_pred_vk_bytes,
        new_birth_predicates,
        new_death_predicates,
        genesis_address,
        recipient,
        &ledger,
        rng,
    )
    .unwrap();

    transactions.push(transaction);

    let transaction_ids: Vec<Vec<u8>> = transactions
        .to_transaction_ids()
        .unwrap()
        .iter()
        .map(|id| id.to_vec())
        .collect();

    let mut merkle_root_bytes = [0u8; 32];
    merkle_root_bytes[..].copy_from_slice(&merkle_root(&transaction_ids));

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;

    let previous_block = ledger.get_latest_block().unwrap();

    // Pseudo mining
    let header = BlockHeader {
        previous_block_hash: previous_block.header.get_hash(),
        merkle_root_hash: MerkleRootHash(merkle_root_bytes),
        time,
        difficulty_target: consensus.get_block_difficulty(&previous_block.header, time),
        nonce: 0, // TODO integrate with actual miner nonce generation
    };

    let block = Block {
        header,
        transactions: transactions.clone(),
    };

    (new_coinbase_records, block)
}

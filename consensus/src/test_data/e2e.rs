use super::*;
use crate::{
    miner::{MemoryPool, Miner},
    ConsensusParameters,
};
use snarkos_dpc::base_dpc::{record::DPCRecord, record_payload::RecordPayload};
use snarkos_genesis::GenesisBlock;
use snarkos_models::{
    dpc::{DPCScheme, Record},
    genesis::Genesis,
};
use snarkos_objects::{dpc::DPCTransactions, Account, AccountPublicKey, Block, BlockHeader};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use once_cell::sync::Lazy;
use rand::Rng;
use std::{
    fs::File,
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
};

/// Helper providing pre-calculated data for e2e tests
pub static DATA: Lazy<TestData> = Lazy::new(|| load_test_data());

pub static GENESIS_BLOCK_HEADER_HASH: Lazy<[u8; 32]> = Lazy::new(|| genesis().header.get_hash().0);

pub static BLOCK_1: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block_1].unwrap());
pub static BLOCK_1_HEADER_HASH: Lazy<[u8; 32]> = Lazy::new(|| DATA.block_1.header.get_hash().0);

pub static BLOCK_2: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block_2].unwrap());
pub static BLOCK_2_HEADER_HASH: Lazy<[u8; 32]> = Lazy::new(|| DATA.block_2.header.get_hash().0);

pub static TRANSACTION_1: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block_1.transactions.0[0]].unwrap());
pub static TRANSACTION_2: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block_2.transactions.0[0]].unwrap());

// Alternative blocks used for testing syncs and rollbacks
pub static ALTERNATIVE_BLOCK_1: Lazy<Vec<u8>> = Lazy::new(|| {
    let alternative_block_1 = Block {
        header: DATA.alternative_block_1_header.clone(),
        transactions: DATA.block_1.transactions.clone(),
    };

    to_bytes![alternative_block_1].unwrap()
});

pub static ALTERNATIVE_BLOCK_2: Lazy<Vec<u8>> = Lazy::new(|| {
    let alternative_block_2 = Block {
        header: DATA.alternative_block_2_header.clone(),
        transactions: DATA.block_2.transactions.clone(),
    };

    to_bytes![alternative_block_2].unwrap()
});

pub fn genesis() -> Block<Tx> {
    let genesis_block: Block<Tx> = FromBytes::read(GenesisBlock::load_bytes().as_slice()).unwrap();

    genesis_block
}

pub struct TestData {
    pub block_1: Block<Tx>,
    pub block_2: Block<Tx>,
    pub records_1: Vec<DPCRecord<Components>>,
    pub records_2: Vec<DPCRecord<Components>>,
    pub alternative_block_1_header: BlockHeader,
    pub alternative_block_2_header: BlockHeader,
}

impl ToBytes for TestData {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.block_1.write(&mut writer)?;

        self.block_2.write(&mut writer)?;

        writer.write(&(self.records_1.len() as u64).to_le_bytes())?;
        self.records_1.write(&mut writer)?;

        writer.write(&(self.records_2.len() as u64).to_le_bytes())?;
        self.records_2.write(&mut writer)?;

        self.alternative_block_1_header.write(&mut writer)?;
        self.alternative_block_2_header.write(&mut writer)?;

        Ok(())
    }
}

impl FromBytes for TestData {
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let block_1: Block<Tx> = FromBytes::read(&mut reader)?;

        let block_2: Block<Tx> = FromBytes::read(&mut reader)?;

        let len = u64::read(&mut reader)? as usize;
        let records_1 = (0..len)
            .map(|_| FromBytes::read(&mut reader))
            .collect::<Result<Vec<_>, _>>()?;

        let len = u64::read(&mut reader)? as usize;
        let records_2 = (0..len)
            .map(|_| FromBytes::read(&mut reader))
            .collect::<Result<Vec<_>, _>>()?;

        let alternative_block_1_header: BlockHeader = FromBytes::read(&mut reader)?;
        let alternative_block_2_header: BlockHeader = FromBytes::read(&mut reader)?;

        Ok(Self {
            block_1,
            block_2,
            records_1,
            records_2,
            alternative_block_1_header,
            alternative_block_2_header,
        })
    }
}

fn load_test_data() -> TestData {
    if let Ok(test_data) = TestData::read(&include_bytes!("precomputed_data")[..]) {
        test_data
    } else {
        setup_and_store_test_data()
    }
}

fn setup_and_store_test_data() -> TestData {
    // get the params
    let parameters = &FIXTURE.parameters;
    let ledger = FIXTURE.ledger();
    let [miner_acc, acc_1, _] = FIXTURE.test_accounts.clone();
    let mut rng = FIXTURE.rng.clone();
    let consensus = TEST_CONSENSUS;

    // setup the miner
    let miner = Miner::new(miner_acc.public_key.clone(), consensus.clone());
    let mut memory_pool = MemoryPool::new();

    // mine an empty block
    let (block_1, coinbase_records) = mine_block(&miner, &ledger, &parameters, &consensus, &mut memory_pool, vec![]);

    // make a tx which spends 10 to the BaseDPCComponents receiver
    let (_records_1, tx_1) = send(
        &ledger,
        &parameters,
        &miner_acc,
        coinbase_records.clone(),
        &acc_1.public_key,
        10,
        &mut rng,
    );

    // mine the block
    let (block_2, coinbase_records_2) =
        mine_block(&miner, &ledger, &parameters, &consensus, &mut memory_pool, vec![tx_1]);

    // Find alternative conflicting/late blocks

    let alternative_block_1_header = miner
        .find_block(
            &block_1.transactions,
            &ledger.get_block_header(&block_1.header.previous_block_hash).unwrap(),
        )
        .unwrap();
    let alternative_block_2_header = miner
        .find_block(&block_2.transactions, &alternative_block_1_header)
        .unwrap();

    let test_data = TestData {
        block_1,
        block_2,
        records_1: coinbase_records,
        records_2: coinbase_records_2,
        alternative_block_1_header,
        alternative_block_2_header,
    };

    // TODO (howardwu): Remove file generation here in favor of out of scope generation.
    const TEST_DATA_FILE: &str = "precomputed_data";
    let file = std::io::BufWriter::new(File::create(PathBuf::from(TEST_DATA_FILE)).expect("could not open file"));
    test_data.write(file).expect("could not write to file");
    test_data
}

fn mine_block(
    miner: &Miner,
    ledger: &MerkleTreeLedger,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    consensus: &ConsensusParameters,
    memory_pool: &mut MemoryPool<Tx>,
    txs: Vec<Tx>,
) -> (Block<Tx>, Vec<DPCRecord<Components>>) {
    let transactions = DPCTransactions(txs);

    let (previous_block_header, transactions, coinbase_records) =
        miner.establish_block(&parameters, ledger, &transactions).unwrap();

    let header = miner.find_block(&transactions, &previous_block_header).unwrap();

    let block = Block { header, transactions };

    let old_block_height = ledger.get_latest_block_height();

    // add it to the chain
    consensus
        .receive_block(&parameters, ledger, memory_pool, &block)
        .unwrap();

    let new_block_height = ledger.get_latest_block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    // Duplicate blocks dont do anything
    consensus
        .receive_block(&parameters, ledger, memory_pool, &block)
        .unwrap();
    let new_block_height = ledger.get_latest_block_height();
    assert_eq!(old_block_height + 1, new_block_height);

    (block, coinbase_records)
}

/// Spends some value from inputs owned by the sender, to the receiver,
/// and pays back whatever we are left with.
fn send<R: Rng>(
    ledger: &MerkleTreeLedger,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    from: &Account<Components>,
    inputs: Vec<DPCRecord<Components>>,
    receiver: &AccountPublicKey<Components>,
    amount: u64,
    rng: &mut R,
) -> (Vec<DPCRecord<Components>>, Tx) {
    let mut sum = 0;
    for inp in &inputs {
        sum += inp.value();
    }
    assert!(sum >= amount, "not enough balance in inputs");
    let change = sum - amount;

    let in_predicates = vec![FIXTURE.predicate.clone(); NUM_INPUT_RECORDS];
    let out_predicates = vec![FIXTURE.predicate.clone(); NUM_OUTPUT_RECORDS];

    let to = vec![receiver.clone(), from.public_key.clone()];
    let values = vec![amount, change];
    let output = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];
    let dummy_flags = vec![false; NUM_OUTPUT_RECORDS];

    let from = vec![from.private_key.clone(); NUM_INPUT_RECORDS];
    ConsensusParameters::create_transaction(
        parameters,
        inputs,
        from,
        to,
        in_predicates,
        out_predicates,
        dummy_flags,
        values,
        output,
        [0u8; 32], // TODO: Should we set these to anything?
        [0u8; 32],
        &ledger,
        rng,
    )
    .unwrap()
}

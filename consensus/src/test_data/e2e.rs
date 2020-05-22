use super::*;
use crate::{
    miner::{MemoryPool, Miner},
    ConsensusParameters,
};
use snarkos_dpc::base_dpc::{record::DPCRecord, record_payload::PaymentRecordPayload};
use snarkos_models::dpc::{DPCScheme, Record};
use snarkos_objects::{
    dpc::DPCTransactions,
    Account,
    AccountPublicKey,
    Block,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
    PedersenMerkleRootHash,
    ProofOfSuccinctWork,
};
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

pub static BLOCK_1: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block1].unwrap());
pub static BLOCK_1_HEADER_HASH: Lazy<[u8; 32]> = Lazy::new(|| DATA.block1.header.get_hash().0);

pub static BLOCK_2: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block2].unwrap());
pub static BLOCK_2_HEADER_HASH: Lazy<[u8; 32]> = Lazy::new(|| DATA.block2.header.get_hash().0);

pub static TRANSACTION_1: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block1.transactions.0[0]].unwrap());
pub static TRANSACTION_2: Lazy<Vec<u8>> = Lazy::new(|| to_bytes![DATA.block2.transactions.0[0]].unwrap());

pub fn genesis() -> Block<Tx> {
    let header = BlockHeader {
        previous_block_hash: BlockHeaderHash([0u8; 32]),
        merkle_root_hash: MerkleRootHash([0u8; 32]),
        time: 0,
        difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
        nonce: 0,
        pedersen_merkle_root_hash: PedersenMerkleRootHash([0u8; 32]),
        proof: ProofOfSuccinctWork::default(),
    };

    let genesis_block = Block {
        header,
        transactions: DPCTransactions::new(),
    };

    genesis_block
}

pub struct TestData {
    pub block1: Block<Tx>,
    pub block2: Block<Tx>,
    pub records1: Vec<DPCRecord<Components>>,
    pub records2: Vec<DPCRecord<Components>>,
}

impl ToBytes for TestData {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.block1.write(&mut writer)?;

        self.block2.write(&mut writer)?;

        writer.write(&(self.records1.len() as u64).to_le_bytes())?;
        self.records1.write(&mut writer)?;

        writer.write(&(self.records2.len() as u64).to_le_bytes())?;
        self.records2.write(&mut writer)?;

        Ok(())
    }
}

impl FromBytes for TestData {
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let block1: Block<Tx> = FromBytes::read(&mut reader)?;

        let block2: Block<Tx> = FromBytes::read(&mut reader)?;

        let len = u64::read(&mut reader)? as usize;
        let records1 = (0..len)
            .map(|_| FromBytes::read(&mut reader))
            .collect::<Result<Vec<_>, _>>()?;

        let len = u64::read(&mut reader)? as usize;
        let records2 = (0..len)
            .map(|_| FromBytes::read(&mut reader))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            block1,
            block2,
            records1,
            records2,
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
    let [miner_acc, acc1, _] = FIXTURE.test_accounts.clone();
    let mut rng = FIXTURE.rng.clone();
    let consensus = TEST_CONSENSUS.clone();

    // setup the miner
    let miner = Miner::new(miner_acc.public_key.clone(), consensus.clone(), POSW_PP.0.clone());
    let mut memory_pool = MemoryPool::new();

    // mine an empty block
    let (block1, coinbase_records) = mine_block(
        &miner,
        &ledger,
        &parameters,
        &consensus,
        &mut memory_pool,
        &mut rng,
        vec![],
    );

    // make a tx which spends 10 to the BaseDPCComponentsreceiver
    let (_records1, tx1) = send(
        &ledger,
        &parameters,
        &miner_acc,
        coinbase_records.clone(),
        &acc1.public_key,
        10,
        &mut rng,
    );

    // mine the block
    let (block2, coinbase_records2) = mine_block(
        &miner,
        &ledger,
        &parameters,
        &consensus,
        &mut memory_pool,
        &mut rng,
        vec![tx1],
    );

    let test_data = TestData {
        block1,
        block2,
        records1: coinbase_records,
        records2: coinbase_records2,
    };

    // TODO (howardwu): Remove file generation here in favor of out of scope generation.
    const TEST_DATA_FILE: &str = "precomputed_data";
    let file = std::io::BufWriter::new(File::create(PathBuf::from(TEST_DATA_FILE)).expect("could not open file"));
    test_data.write(file).expect("could not write to file");
    test_data
}

fn mine_block<R: Rng>(
    miner: &Miner,
    ledger: &MerkleTreeLedger,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    consensus: &ConsensusParameters,
    memory_pool: &mut MemoryPool<Tx>,
    rng: &mut R,
    txs: Vec<Tx>,
) -> (Block<Tx>, Vec<DPCRecord<Components>>) {
    let transactions = DPCTransactions(txs);

    let (previous_block_header, transactions, coinbase_records) =
        miner.establish_block(&parameters, ledger, &transactions).unwrap();

    let header = miner.find_block(&transactions, &previous_block_header, rng).unwrap();

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
        sum += inp.payload().balance;
    }
    assert!(sum >= amount, "not enough balance in inputs");
    let change = sum - amount;

    let in_predicates = vec![FIXTURE.predicate.clone(); NUM_INPUT_RECORDS];
    let out_predicates = vec![FIXTURE.predicate.clone(); NUM_OUTPUT_RECORDS];

    let to = vec![receiver.clone(), from.public_key.clone()];
    let output = vec![
        PaymentRecordPayload {
            balance: amount,
            lock: 0,
        },
        PaymentRecordPayload {
            balance: change,
            lock: 0,
        },
    ];
    let dummy_flags = vec![false, false];

    let from = vec![from.private_key.clone(); NUM_INPUT_RECORDS];
    ConsensusParameters::create_transaction(
        parameters,
        inputs,
        from,
        to,
        in_predicates,
        out_predicates,
        dummy_flags,
        output,
        [0u8; 32], // TODO: Should we set these to anything?
        [0u8; 32],
        &ledger,
        rng,
    )
    .unwrap()
}

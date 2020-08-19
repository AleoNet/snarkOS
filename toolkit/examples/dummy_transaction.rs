use snarkos_toolkit::transaction::create_dummy_transaction;
use snarkos_utilities::{to_bytes, ToBytes};

use rand::thread_rng;

pub fn main() {
    let rng = &mut thread_rng();

    let (transaction, records) = create_dummy_transaction(rng).unwrap();

    let encoded_transaction = hex::encode(to_bytes![transaction].unwrap());
    println!("transaction: {}", encoded_transaction);

    for (i, record) in records.iter().enumerate() {
        println!("record {}: {}", i, hex::encode(to_bytes![record].unwrap()));
    }
}

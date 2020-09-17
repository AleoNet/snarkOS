use snarkos_toolkit::transaction::create_dummy_transaction;
use snarkos_utilities::{to_bytes, ToBytes};

use rand::thread_rng;
use std::{env::current_dir, fs::File, io::Write};

pub fn main() {
    let rng = &mut thread_rng();

    let network_id = 1;

    let (transaction, _records) = create_dummy_transaction(network_id, rng).unwrap();

    let encoded_transaction = hex::encode(to_bytes![transaction].unwrap());
    println!("{}", encoded_transaction);

    // Write the transaction to a file
    //let mut file_name = current_dir().unwrap();
    //file_name.push("dummy_transaction.txt");

    let file_name = "/home/ubuntu/snarkOS/toolkit/dummy_transaction.txt";

    std::fs::remove_file(file_name).unwrap();

    let mut file = File::create(file_name).unwrap();
    file.write_all(encoded_transaction.as_bytes()).unwrap();
}

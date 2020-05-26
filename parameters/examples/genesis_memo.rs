use snarkos_errors::dpc::DPCError;

use rand::{thread_rng, Rng};
use std::{
    fs::File,
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup() -> Result<Vec<u8>, DPCError> {
    let rng = &mut thread_rng();
    let genesis_memo: [u8; 32] = rng.gen();

    println!("genesis_memo\n\tsize - {}", genesis_memo.len());
    Ok(genesis_memo.to_vec())
}

pub fn store(path: &PathBuf, bytes: &Vec<u8>) -> IoResult<()> {
    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let genesis_memo = setup().unwrap();
    store(&PathBuf::from("genesis_memo"), &genesis_memo).unwrap();
}

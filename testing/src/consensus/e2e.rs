// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use snarkos_models::genesis::Genesis;
use snarkos_objects::{Block, BlockHeader};
use snarkos_parameters::GenesisBlock;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};
use snarkvm_dpc::base_dpc::{instantiated::*, record::DPCRecord};

use once_cell::sync::Lazy;
use std::io::{Read, Result as IoResult, Write};

/// Helper providing pre-calculated data for e2e tests
pub static DATA: Lazy<TestData> = Lazy::new(load_test_data);

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

        writer.write_all(&(self.records_1.len() as u64).to_le_bytes())?;
        self.records_1.write(&mut writer)?;

        writer.write_all(&(self.records_2.len() as u64).to_le_bytes())?;
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
    TestData::read(&include_bytes!("test_data")[..]).unwrap()
}

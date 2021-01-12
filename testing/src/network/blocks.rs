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

use snarkvm_dpc::instantiated::Tx;
use snarkvm_objects::Block;
use snarkvm_utilities::{FromBytes, Read, ToBytes, Write};
use std::io::Result;

#[derive(Debug)]
pub struct TestBlocks(Vec<Block<Tx>>);

impl TestBlocks {
    pub fn new(blocks: Vec<Block<Tx>>) -> Self {
        TestBlocks(blocks)
    }

    pub fn load() -> Self {
        TestBlocks::read(&include_bytes!("test_blocks")[..]).unwrap()
    }

    // TODO: implement Deref?
    pub fn inner(&self) -> Vec<Block<Tx>> {
        self.0.clone()
    }
}

impl ToBytes for TestBlocks {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> Result<()> {
        for block in &self.0 {
            // Clone is necessary here, otherwise weird things happen.
            let block = block.clone();
            block.write(&mut writer)?;
        }

        Ok(())
    }
}

impl FromBytes for TestBlocks {
    fn read<R: Read>(mut reader: R) -> Result<Self> {
        let mut blocks = vec![];

        // Hardcoded for now as the trait doesn't allow for an N.
        for i in 0..10 {
            let block: Block<Tx> = FromBytes::read(&mut reader)?;
            blocks.push(block);
        }

        Ok(TestBlocks::new(blocks))
    }
}

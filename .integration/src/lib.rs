// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use snarkos_node_cdn::sync_ledger_with_cdn;
    use snarkos_node_ledger::Ledger;
    use snarkvm::prelude::{Block, ConsensusMemory, FromBytes, Network, Testnet3};

    use tracing_test::traced_test;

    type CurrentNetwork = Testnet3;

    const TEST_BASE_URL: &str = "https://testnet3.blocks.aleo.org/phase2";

    #[test]
    #[traced_test]
    fn test_sync_ledger_with_cdn_0_to_tip() {
        // Initialize the genesis block.
        let genesis = Block::<CurrentNetwork>::read_le(CurrentNetwork::genesis_bytes()).unwrap();
        // Initialize the ledger.
        let ledger = Ledger::<_, ConsensusMemory<_>>::load(genesis, None).unwrap();
        // Perform the sync.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let completed_height = sync_ledger_with_cdn(TEST_BASE_URL, ledger.clone()).await.unwrap();
            assert_eq!(completed_height, ledger.latest_height());
        });
    }
}

// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use aleo_std::StorageMode;
    use snarkos_node_cdn::sync_ledger_with_cdn;
    use snarkvm::prelude::{
        block::Block,
        store::helpers::memory::ConsensusMemory,
        FromBytes,
        Ledger,
        MainnetV0,
        Network,
    };

    use tracing_test::traced_test;

    type CurrentNetwork = MainnetV0;

    const TEST_BASE_URL: &str = "https://testnet3.blocks.aleo.org/phase3";

    #[test]
    #[traced_test]
    fn test_sync_ledger_with_cdn_0_to_tip() {
        // Initialize the genesis block.
        let genesis = Block::<CurrentNetwork>::read_le(CurrentNetwork::genesis_bytes()).unwrap();
        // Initialize the ledger.
        let ledger = Ledger::<_, ConsensusMemory<_>>::load(genesis, StorageMode::Production).unwrap();
        // Perform the sync.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let completed_height =
                sync_ledger_with_cdn(TEST_BASE_URL, ledger.clone(), Default::default()).await.unwrap();
            assert_eq!(completed_height, ledger.latest_height());
        });
    }
}

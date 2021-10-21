// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use snarkos_metrics::wrapped_mpsc;

use crate::consensus::ConsensusMessageWrapped;

use super::*;

impl ConsensusInner {
    /// initialize genesis block if neccesary and catches up the chain
    async fn init(&mut self) -> Result<()> {
        let canon = self.storage.canon().await?;
        // no blocks present/genesis situation
        if canon.is_empty() {
            // no blocks
            let hash = self.public.genesis_block.header.hash();
            let block = self.public.genesis_block.clone();
            self.storage.insert_block(&block).await?;

            self.commit_block(&hash, &block).await?;
        }

        Ok(())
    }

    pub(in crate::consensus) async fn agent(mut self, mut receiver: wrapped_mpsc::Receiver<ConsensusMessageWrapped>) {
        self.init()
            .await
            .expect("failed to initialize ledger & storage with genesis block");

        while let Some((message, response)) = receiver.recv().await {
            match message {
                ConsensusMessage::ReceiveTransaction(transaction) => {
                    let ret = self.receive_transaction(transaction).await;
                    response.send(Box::new(ret)).ok();
                }
                ConsensusMessage::VerifyTransactions(transactions) => {
                    let out = match self.verify_transactions(transactions).await {
                        Ok(out) => out,
                        Err(e) => {
                            error!(
                                "failed to validate transactions -- note this does not mean the transactions were valid or invalid: {:?}",
                                e
                            );
                            false
                        }
                    };
                    response.send(Box::new(out)).ok();
                }
                ConsensusMessage::ReceiveBlock(block) => match self.receive_block(&block).await {
                    Ok(()) => {
                        response.send(Box::new(true)).ok();
                    }
                    Err(e) => {
                        match e {
                            ConsensusError::InvalidBlock(e) => {
                                debug!("failed receiving block: {:?}", e);
                            }
                            e => {
                                debug!("failed receiving block: {:?}", e);
                            }
                        }
                        response.send(Box::new(false)).ok();
                    }
                },
                ConsensusMessage::FetchMemoryPool(size) => {
                    let out: Vec<SerialTransaction> =
                        self.memory_pool.get_candidates(size).into_iter().cloned().collect();
                    response.send(Box::new(out)).ok();
                }
                ConsensusMessage::CreateTransaction(request) => {
                    let out = self.create_transaction(*request).await;
                    response.send(Box::new(out)).ok();
                }
                ConsensusMessage::CreatePartialTransaction(request) => {
                    let out = self.create_partial_transaction(request).await;
                    response.send(Box::new(out)).ok();
                }
                ConsensusMessage::ForceDecommit(hash) => {
                    let out = self.decommit_ledger_block(&hash).await;
                    response.send(Box::new(out)).ok();
                }
                ConsensusMessage::FastForward() => {
                    let out = self.try_to_fast_forward().await;
                    response.send(Box::new(out)).ok();
                }
                #[cfg(feature = "test")]
                ConsensusMessage::Reset() => {
                    response
                        .send(Box::new(
                            async {
                                self.ledger.clear();
                                self.storage.reset().await?;
                                self.init().await
                            }
                            .await,
                        ))
                        .ok();
                }
            }
        }
    }
}

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

use super::CurrentNetwork;

use snarkvm::prelude::{Block, Plaintext, Record, ViewKey};

use anyhow::{bail, Result};
use clap::Parser;
use std::str::FromStr;

// TODO (raychu86): Figure out what to do with this naive scan. This scan currently does not check if records are already spent.
/// Scan the snarkOS node for records.
#[derive(Debug, Parser)]
pub struct Scan {
    /// The view key used to scan the snarkOS node.
    #[clap(short = 'v', long, help = "The view key used to scan the snarkOS node")]
    pub view_key: String,

    #[clap(long, help = "The block to start scanning", default_value = "0")]
    pub start: u32,

    #[clap(long, help = "The block to stop scanning")]
    pub end: Option<u32>,

    #[clap(short = 'r', long, help = "Scan in reverse order")]
    pub reverse: bool,

    #[clap(long, help = "Maximum number of gates to search for")]
    pub max_gates: Option<u64>,

    #[clap(long, help = "The URL to scan blocks from")]
    endpoint: String,
}

impl Scan {
    pub fn parse(self) -> Result<String> {
        // Derive the view key.
        let view_key = ViewKey::<CurrentNetwork>::from_str(&self.view_key)?;

        // Get the latest height
        let endpoint = format!("{}/testnet3/latest/height", self.endpoint);
        let latest_height = u32::from_str(&ureq::get(&endpoint).call()?.into_string()?)?;

        // Find the end height.
        let end = match self.end {
            Some(height) => {
                if height > latest_height {
                    bail!("❌️  The specified end height exceeded the latest block height of {latest_height}");
                }
                height
            }
            None => {
                if self.start == 0 {
                    println!("⚠️  Attention - Scanning the entire chain. This may take a few minutes...\n");
                }
                latest_height
            }
        };

        // Fetch the records from the network.
        let records = Self::fetch_records(&view_key, self.endpoint, self.start, end, self.reverse, self.max_gates)?;

        // Output the decrypted records associated with the view key.
        if records.is_empty() {
            Ok("No records found".to_string())
        } else {
            Ok(serde_json::to_string_pretty(&records)?.replace("\\n", ""))
        }
    }

    // TODO (raychu86): Make these unspent records.
    /// Fetch owned records from the endpoint.
    pub fn fetch_records(
        view_key: &ViewKey<CurrentNetwork>,
        endpoint: String,
        start_height: u32,
        end_height: u32,
        reverse: bool,
        max_gates: Option<u64>,
    ) -> Result<Vec<Record<CurrentNetwork, Plaintext<CurrentNetwork>>>> {
        // Check the bounds of the request.
        if start_height > end_height {
            bail!("Invalid block range");
        }

        // Create a predicate for reverse and forward search.
        let in_range = |cursor, reverse| if reverse { cursor > start_height } else { cursor < end_height };

        // Derive the x-coordinate of the address corresponding to the given view key.
        let address_x_coordinate = view_key.to_address().to_x_coordinate();

        const MAX_BLOCK_RANGE: u32 = 50;

        let mut records = Vec::new();

        let mut gates = 0u64;

        // Scan the endpoint starting from the start height or if reverse mode, the end height.
        let mut cursor = if reverse { start_height } else { end_height };
        while in_range(cursor, reverse) {
            let num_blocks_to_request = if reverse {
                std::cmp::min(MAX_BLOCK_RANGE, start_height.saturating_add(cursor))
            } else {
                std::cmp::min(MAX_BLOCK_RANGE, end_height.saturating_sub(cursor))
            };

            let (request_start, request_end) = if reverse {
                (cursor.saturating_sub(num_blocks_to_request), cursor)
            } else {
                (cursor, cursor.saturating_add(num_blocks_to_request))
            };

            // Establish the endpoint.
            let endpoint = format!("{endpoint}/testnet3/blocks?start={request_start}&end={request_end}");

            // Fetch blocks
            let blocks: Vec<Block<CurrentNetwork>> = ureq::get(&endpoint).call()?.into_json()?;

            // Scan the blocks for owned records.
            for block in &blocks {
                for (_, record) in block.records() {
                    // Check if the record is owned by the given view key.
                    if record.is_owner_with_address_x_coordinate(view_key, &address_x_coordinate) {
                        // Decrypt the record.
                        let decrypted_record = record.decrypt(view_key)?;
                        if max_gates.is_some() {
                            // TODO (iamalwaysuncomfortable): Ensure this is only done for unspent records
                            // Sum the number of gates if a maximum number of gates is specified.
                            gates += ***decrypted_record.gates();
                        }
                        records.push(decrypted_record);
                    }
                }
            }

            // Exit the desired number of gates have been found.
            if max_gates.is_some() && gates >= max_gates.unwrap() {
                break;
            }

            cursor = if reverse { request_start } else { request_end };
        }

        Ok(records)
    }
}

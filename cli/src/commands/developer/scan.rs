// Copyright (C) 2019-2023 Aleo Systems Inc.
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
    #[clap(short, long)]
    pub view_key: String,

    /// The block height to start scanning at
    #[clap(long, default_value = "0")]
    pub start: u32,

    /// The block height to stop scanning
    #[clap(long)]
    pub end: Option<u32>,

    /// The endpoint to scan blocks from
    #[clap(long)]
    endpoint: String,
}

impl Scan {
    pub fn parse(self) -> Result<String> {
        // Derive the view key.
        let view_key = ViewKey::<CurrentNetwork>::from_str(&self.view_key)?;

        // Find the end height.
        let end = match self.end {
            Some(height) => height,
            None => {
                // Request the latest block height from the endpoint.
                let endpoint = format!("{}/testnet3/latest/height", self.endpoint);
                let latest_height = u32::from_str(&ureq::get(&endpoint).call()?.into_string()?)?;

                // Print warning message if the user is attempting to scan the whole chain.
                if self.start == 0 {
                    println!("⚠️  Attention - Scanning the entire chain. This may take a few minutes...\n");
                }

                latest_height
            }
        };

        // Fetch the records from the network.
        let records = Self::fetch_records(&view_key, self.endpoint, self.start, end)?;

        // Output the decrypted records associated with the view key.
        if records.is_empty() {
            Ok("No records found".to_string())
        } else {
            println!("⚠️  This list may contain records that have already been spent.\n");

            Ok(serde_json::to_string_pretty(&records)?.replace("\\n", ""))
        }
    }

    // TODO (raychu86):Account for spent records.
    /// Fetch owned records from the endpoint.
    pub fn fetch_records(
        view_key: &ViewKey<CurrentNetwork>,
        endpoint: String,
        start_height: u32,
        end_height: u32,
    ) -> Result<Vec<Record<CurrentNetwork, Plaintext<CurrentNetwork>>>> {
        // Check the bounds of the request.
        if start_height > end_height {
            bail!("Invalid block range");
        }

        // Derive the x-coordinate of the address corresponding to the given view key.
        let address_x_coordinate = view_key.to_address().to_x_coordinate();

        const MAX_BLOCK_RANGE: u32 = 50;

        let mut records = Vec::new();

        // Scan the endpoint starting from the start height
        let mut request_start = start_height;
        while request_start <= end_height {
            let num_blocks_to_request =
                std::cmp::min(MAX_BLOCK_RANGE, end_height.saturating_sub(request_start).saturating_add(1));
            let request_end = request_start.saturating_add(num_blocks_to_request);

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
                        records.push(record.decrypt(view_key)?);
                    }
                }
            }

            request_start = request_start.saturating_add(num_blocks_to_request);
        }

        Ok(records)
    }
}

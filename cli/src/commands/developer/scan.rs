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

use super::Network;

use snarkvm::prelude::{Block, ViewKey};

use anyhow::Result;
use clap::Parser;
use std::str::FromStr;

// TODO (raychu86): Figure out what to do with this naive scan. This scan currently does not check if records are already spent.
/// Scan the snarkOS node for records.
#[derive(Debug, Parser)]
pub struct Scan {
    /// The view key used to scan the snarkOS node.
    #[clap(short = 'v', long, help = "The view key used to scan the snarkOS node")]
    pub view_key: String,

    #[clap(long, help = "The block to start scanning")]
    pub start: u32,
    #[clap(long, help = "The block to stop scanning")]
    pub end: u32,

    #[clap(long, help = "The URL to scan blocks from")]
    endpoint: String,
}

impl Scan {
    pub fn parse(self) -> Result<String> {
        // Fetch the endpoint.
        let endpoint = format!("{}/testnet3/blocks?start={}&end={}", self.endpoint, self.start, self.end);

        // Derive the view key.
        let view_key = ViewKey::<Network>::from_str(&self.view_key)?;

        // Derive the x-coordinate of the address corresponding to the given view key.
        let address_x_coordinate = view_key.to_address().to_x_coordinate();

        // Get blocks.
        let blocks: Vec<Block<Network>> = ureq::get(&endpoint).call()?.into_json()?;

        let mut records = Vec::new();

        // Scan the given blocks for records.
        for block in &blocks {
            for (_, record) in block.records() {
                // Check if the record is owned by the given view key.
                if record.is_owner_with_address_x_coordinate(&view_key, &address_x_coordinate) {
                    // Decrypt the record.
                    records.push(record.decrypt(&view_key)?);
                }
            }
        }

        // Output the decrypted records associated with the view key.
        if records.is_empty() {
            Ok("No records found".to_string())
        } else {
            Ok(serde_json::to_string_pretty(&records)?.replace("\\n", ""))
        }
    }
}

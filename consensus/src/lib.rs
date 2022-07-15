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

#[macro_use]
extern crate tracing;

// pub mod reference;

// TODO (raychu86): Move this declaration of account.
pub mod account;

mod rewards;

mod round;
mod validator;
mod validators;

use crate::{round::Round, validators::Validators};
use snarkvm::prelude::Network;

#[derive(Copy, Clone, Debug)]
pub enum Status {
    /// The round is running.
    Running,
    /// The round is aborting.
    Aborting,
    /// The round succeeded.
    Completed,
    /// The round failed.
    Failed,
}

/// The consensus struct contains state that is tracked by all validators in the network.
pub struct Consensus<N: Network> {
    /// The current round of consensus.
    round: Round<N>,
    /// The current validators in the network.
    validators: Validators<N>,
}

impl<N: Network> Consensus<N> {
    /// Initializes a new instance of consensus.
    pub fn new(round: Round<N>) -> Self {
        Self {
            round,
            validators: Validators::new(),
        }
    }

    /// Returns the latest round.
    pub const fn latest_round(&self) -> &Round<N> {
        &self.round
    }

    /// Returns the current validators.
    pub const fn validators(&self) -> &Validators<N> {
        &self.validators
    }
}

// TODO (raychu86): Remove this use of genesis block generation.
use snarkvm::{
    circuit::Aleo,
    compiler::{Process, Program, Transition},
    console::{
        account::{Address, PrivateKey, ViewKey},
        program::{Identifier, Value},
    },
    {Block, BlockHeader, Transaction, Transactions},
};
use std::str::FromStr;

pub fn genesis_block<N: Network, A: Aleo<Network = N, BaseField = N::Field>>() -> anyhow::Result<Block<N>> {
    // Initialize a new program.
    let program = Program::<N>::from_str(
        r"program stake.aleo;

  record stake:
    owner as address.private;
    gates as u64.private;

  function initialize:
    input r0 as address.private;
    input r1 as u64.private;
    cast r0 r1 into r2 as stake.record;
    output r2 as stake.record;",
    )?;

    // Declare the function name.
    let function_name = Identifier::from_str("initialize")?;

    // TODO (howardwu): Switch this to a remotely loaded SRS.
    let rng = &mut snarkvm::utilities::test_crypto_rng_fixed();

    // Initialize a new caller account.
    let caller_private_key = PrivateKey::<N>::new(rng)?;
    let _caller_view_key = ViewKey::try_from(&caller_private_key)?;
    let caller = Address::try_from(&caller_private_key)?;

    // Declare the input value.
    let r0 = Value::<N>::from_str(&format!("{caller}"))?;
    let r1 = Value::<N>::from_str("1_000_000_000_000_000_u64")?;

    // Construct the process.
    let mut process = Process::<N, A>::new()?;
    // Add the program to the process.
    process.add_program(&program)?;

    // Authorize the function call.
    let authorization = process.authorize(&caller_private_key, program.id(), function_name, &[r0.clone(), r1.clone()], rng)?;

    // Execute the request.
    let (_response, execution) = process.execute(authorization, rng)?;

    let transitions = execution.to_vec();
    let transaction = Transaction::execute(transitions)?;

    // Prepare the components.
    let header = BlockHeader::<N>::genesis();
    let transactions = Transactions::from(&[transaction])?;
    let previous_hash = N::BlockHash::default();

    // Construct the block.
    let block = Block::from(previous_hash, header, transactions)?;

    Ok(block)
}

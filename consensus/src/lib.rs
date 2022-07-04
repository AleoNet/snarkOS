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

// pub mod reference;

mod block;
pub use block::*;

mod message;
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

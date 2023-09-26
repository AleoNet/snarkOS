// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use snarkvm::{
    console::{network::Network, types::Field},
    prelude::{error, FromBytes, ToBytes},
};

use anyhow::{bail, ensure, Result};
use indexmap::IndexSet;
use std::{
    collections::{BTreeMap, HashSet},
    io::{Read, Result as IoResult, Write},
};

/// The maximum number of GC rounds.
/// Note: This is a soft limit, to ensure the round locators are not too large.
pub const MAX_ROUNDS: usize = 128;
/// The maximum number of certificates per round.
/// Note: This is a soft limit, to ensure the round locators are not too large.
pub const MAX_CERTIFICATES_PER_ROUND: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoundLocators<N: Network> {
    /// The map of rounds to certificate IDs.
    certificate_ids: BTreeMap<u64, IndexSet<Field<N>>>,
}

impl<N: Network> RoundLocators<N> {
    /// Initializes a new round locators.
    pub const fn new(certificate_ids: BTreeMap<u64, IndexSet<Field<N>>>) -> Self {
        Self { certificate_ids }
    }

    /// Returns the map of rounds to certificate IDs.
    pub const fn certificate_ids(&self) -> &BTreeMap<u64, IndexSet<Field<N>>> {
        &self.certificate_ids
    }

    /// Returns `true` if the given round and certificate ID exists.
    pub fn contains_certificate_id(&self, round: u64, certificate_id: Field<N>) -> bool {
        self.certificate_ids.get(&round).map_or(false, |certificate_ids| certificate_ids.contains(&certificate_id))
    }

    /// Ensures the round locators are well-formed.
    ///
    /// Note: To ensure flexibility in node design, we do not enforce a strict requirement
    /// on the number of rounds. However, we do enforce a maximum number of rounds.
    pub fn ensure_is_well_formed(&self) -> Result<()> {
        // Ensure the number of rounds does not exceed the maximum number of GC rounds.
        if self.certificate_ids.len() > MAX_ROUNDS {
            bail!("The number of rounds exceeds the maximum number of GC rounds")
        }

        // Initialize the previous round.
        let mut previous_round = self.certificate_ids.keys().next().copied().unwrap_or(0).saturating_sub(1);

        // Initialize the set of unique certificate IDs.
        let mut unique = HashSet::new();

        // Iterate over the rounds.
        for (round, certificate_ids) in &self.certificate_ids {
            // Ensure the rounds are sequential.
            ensure!(*round == previous_round.saturating_add(1), "Round locators contains non-sequential rounds");

            // Ensure the rounds are not empty.
            ensure!(!certificate_ids.is_empty(), "Round locators includes an empty round");
            // Ensure the number of certificate IDs does not exceed the maximum number of certificates per round.
            if certificate_ids.len() > MAX_CERTIFICATES_PER_ROUND {
                bail!("The number of certificate IDs exceeds the maximum number of certificates per round")
            }
            // Ensure the certificate IDs are unique.
            if !certificate_ids.into_iter().all(|x| unique.insert(x)) {
                bail!("Round locators contains duplicate certificate IDs")
            }

            // Update the previous round.
            previous_round = *round;
        }
        Ok(())
    }
}

impl<N: Network> FromBytes for RoundLocators<N> {
    /// Reads the round locators from the given reader.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the number of rounds.
        let num_rounds = u16::read_le(&mut reader)?;
        // Read the rounds.
        let mut certificate_ids = BTreeMap::new();
        for _ in 0..num_rounds {
            // Read the round.
            let round = u64::read_le(&mut reader)?;
            // Read the number of certificate IDs.
            let num_certificate_ids = u16::read_le(&mut reader)?;
            // Read the certificate IDs.
            let mut ids = IndexSet::new();
            for _ in 0..num_certificate_ids {
                ids.insert(Field::read_le(&mut reader)?);
            }
            // Insert the certificate IDs into the map.
            certificate_ids.insert(round, ids);
        }
        // Return the round locators.
        Ok(Self::new(certificate_ids))
    }
}

impl<N: Network> ToBytes for RoundLocators<N> {
    /// Writes the round locators to the given writer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the number of rounds.
        u16::try_from(self.certificate_ids.len()).map_err(error)?.write_le(&mut writer)?;
        // Write the rounds.
        for (round, certificate_ids) in &self.certificate_ids {
            // Write the round.
            round.write_le(&mut writer)?;
            // Write the number of certificate IDs.
            u16::try_from(certificate_ids.len()).map_err(error)?.write_le(&mut writer)?;
            // Write the certificate IDs.
            for certificate_id in certificate_ids {
                certificate_id.write_le(&mut writer)?;
            }
        }
        // Return success.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type CurrentNetwork = snarkvm::console::network::Testnet3;

    #[test]
    fn test_round_locators() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (1, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
                (2, [4, 5, 6].map(Field::<CurrentNetwork>::from_u8)),
                (3, [7, 8, 9].map(Field::<CurrentNetwork>::from_u8)),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        round_locators.ensure_is_well_formed().unwrap();
    }

    #[test]
    fn test_empty_round_locators() {
        // Initialize the round locators.
        let round_locators = RoundLocators::<CurrentNetwork>::new(BTreeMap::new());
        // Ensure the round locators are well-formed.
        round_locators.ensure_is_well_formed().unwrap();
    }

    #[test]
    fn test_round_locators_with_round_zero_fails() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (0, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
                (1, [4, 5, 6].map(Field::<CurrentNetwork>::from_u8)),
                (2, [7, 8, 9].map(Field::<CurrentNetwork>::from_u8)),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        assert!(round_locators.ensure_is_well_formed().is_err());
    }

    #[test]
    fn test_round_locators_are_sequential() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (1, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
                (3, [4, 5, 6].map(Field::<CurrentNetwork>::from_u8)),
                (4, [7, 8, 9].map(Field::<CurrentNetwork>::from_u8)),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        assert!(round_locators.ensure_is_well_formed().is_err());
    }

    #[test]
    fn test_round_locators_duplicate_certificate_ids() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (0, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
                (1, [4, 5, 6].map(Field::<CurrentNetwork>::from_u8)),
                (2, [7, 8, 9].map(Field::<CurrentNetwork>::from_u8)),
                (3, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        assert!(round_locators.ensure_is_well_formed().is_err());
    }

    #[test]
    fn test_round_locators_empty_round() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (0, vec![1, 2, 3].into_iter().map(Field::<CurrentNetwork>::from_u8).collect::<IndexSet<_>>()),
                (1, vec![4, 5, 6].into_iter().map(Field::<CurrentNetwork>::from_u8).collect::<IndexSet<_>>()),
                (2, vec![7, 8, 9].into_iter().map(Field::<CurrentNetwork>::from_u8).collect::<IndexSet<_>>()),
                (3, vec![].into_iter().collect::<IndexSet<_>>()),
            ]
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        assert!(round_locators.ensure_is_well_formed().is_err());
    }

    #[test]
    fn test_round_locators_too_many_rounds() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            (0..=MAX_ROUNDS)
                .map(|round| {
                    let round = u64::try_from(round).unwrap();
                    let i = round * 3;
                    (round, [i, i + 1, i + 2].map(Field::<CurrentNetwork>::from_u64))
                })
                .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
                .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        assert!(round_locators.ensure_is_well_formed().is_err());
    }

    #[test]
    fn test_round_locators_too_many_certificate_ids() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (0, vec![1, 2, 3].into_iter().map(Field::<CurrentNetwork>::from_u8).collect::<IndexSet<_>>()),
                (1, vec![4, 5, 6].into_iter().map(Field::<CurrentNetwork>::from_u8).collect::<IndexSet<_>>()),
                (2, vec![7, 8, 9].into_iter().map(Field::<CurrentNetwork>::from_u8).collect::<IndexSet<_>>()),
                (3, (0..=MAX_CERTIFICATES_PER_ROUND as u64).map(Field::<CurrentNetwork>::from_u64).collect()),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        assert!(round_locators.ensure_is_well_formed().is_err());
    }

    #[test]
    fn test_round_locators_bytes() {
        // Initialize the round locators.
        let round_locators = RoundLocators::new(
            vec![
                (1, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
                (2, [4, 5, 6].map(Field::<CurrentNetwork>::from_u8)),
                (3, [7, 8, 9].map(Field::<CurrentNetwork>::from_u8)),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        );
        // Ensure the round locators are well-formed.
        round_locators.ensure_is_well_formed().unwrap();

        // Convert the round locators to bytes.
        let bytes = round_locators.to_bytes_le().unwrap();
        // Convert the bytes to round locators.
        let candidate_round_locators = RoundLocators::<CurrentNetwork>::from_bytes_le(&bytes).unwrap();
        // Ensure the round locators are well-formed.
        candidate_round_locators.ensure_is_well_formed().unwrap();

        assert_eq!(round_locators, candidate_round_locators);
    }
}

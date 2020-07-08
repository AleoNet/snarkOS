mod assignment;
mod constraint_system;
mod impl_constraint_var;
mod impl_lc;
mod test_constraint_system;
mod test_fr;

pub use crate::curves::to_field_vec::ToConstraintField;
pub use assignment::*;
pub use constraint_system::{ConstraintSynthesizer, ConstraintSystem, Namespace};
pub use test_constraint_system::TestConstraintSystem;
pub use test_fr::*;

use crate::curves::Field;

use smallvec::SmallVec as StackVec;
use snarkos_errors::serialization::SerializationError;
use snarkos_utilities::serialize::*;
use std::cmp::Ordering;

type SmallVec<F> = StackVec<[(Variable, F); 16]>;

/// Represents a variable in a constraint system.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
pub struct Variable(Index);

impl Variable {
    /// This constructs a variable with an arbitrary index.
    /// Circuit implementations are not recommended to use this.
    pub fn new_unchecked(idx: Index) -> Variable {
        Variable(idx)
    }

    /// This returns the index underlying the variable.
    /// Circuit implementations are not recommended to use this.
    pub fn get_unchecked(&self) -> Index {
        self.0
    }
}

/// Represents the index of either an input variable or auxiliary variable.
#[derive(Copy, Clone, PartialEq, Debug, Eq)]
pub enum Index {
    /// Index of an input variable.
    Input(usize),
    /// Index of an auxiliary (or private) variable.
    Aux(usize),
}

impl PartialOrd for Index {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Index {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Index::Input(ref idx1), Index::Input(ref idx2)) | (Index::Aux(ref idx1), Index::Aux(ref idx2)) => {
                idx1.cmp(idx2)
            }
            (Index::Input(_), Index::Aux(_)) => Ordering::Less,
            (Index::Aux(_), Index::Input(_)) => Ordering::Greater,
        }
    }
}

impl CanonicalSerialize for Index {
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), SerializationError> {
        let inner = match *self {
            Index::Input(inner) => {
                true.serialize(writer)?;
                inner
            }
            Index::Aux(inner) => {
                false.serialize(writer)?;
                inner
            }
        };
        inner.serialize(writer)?;
        Ok(())
    }

    #[inline]
    fn serialized_size(&self) -> usize {
        Self::SERIALIZED_SIZE
    }
}

impl ConstantSerializedSize for Index {
    const SERIALIZED_SIZE: usize = usize::SERIALIZED_SIZE + 1;
    const UNCOMPRESSED_SIZE: usize = Self::SERIALIZED_SIZE;
}

impl CanonicalDeserialize for Index {
    #[inline]
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, SerializationError> {
        let is_input = bool::deserialize(reader)?;
        let inner = usize::deserialize(reader)?;
        Ok(if is_input {
            Index::Input(inner)
        } else {
            Index::Aux(inner)
        })
    }
}

/// This represents a linear combination of some variables, with coefficients
/// in the field `F`.
/// The `(coeff, var)` pairs in a `LinearCombination` are kept sorted according
/// to the index of the variable in its constraint system.
#[derive(Debug, Clone)]
pub struct LinearCombination<F: Field>(pub SmallVec<F>);

/// Either a `Variable` or a `LinearCombination`.
#[derive(Clone, Debug)]
pub enum ConstraintVar<F: Field> {
    /// A wrapper around a `LinearCombination`.
    LC(LinearCombination<F>),
    /// A wrapper around a `Variable`.
    Var(Variable),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn serialize_index() {
        serialize_index_test(true);
        serialize_index_test(false);
    }

    fn serialize_index_test(input: bool) {
        let idx = if input { Index::Input(32) } else { Index::Aux(32) };

        let mut v = vec![];
        idx.serialize(&mut v).unwrap();
        let idx2 = Index::deserialize(&mut &v[..]).unwrap();
        assert_eq!(idx, idx2);
    }
}

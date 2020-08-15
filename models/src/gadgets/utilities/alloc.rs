use crate::{curves::Field, gadgets::r1cs::ConstraintSystem};
use snarkos_errors::gadgets::SynthesisError;

use std::borrow::Borrow;

pub trait AllocBytesGadget<V: ?Sized, F: Field>: Sized
where
    V: Into<Option<Vec<u8>>>,
{
    fn alloc_bytes<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError>;

    fn alloc_bytes_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Self::alloc_bytes(cs, f)
    }

    fn alloc_input_bytes<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError>;

    fn alloc_input_bytes_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Self::alloc_input_bytes(cs, f)
    }
}

pub trait AllocGadget<V: ?Sized, F: Field>: Sized {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError>;

    fn alloc_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Self::alloc(cs, f)
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError>;

    fn alloc_input_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<V>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Self::alloc_input(cs, f)
    }
}

impl<I, F: Field, A: AllocGadget<I, F>> AllocGadget<[I], F> for Vec<A> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[I]>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let mut vec = vec![];
        for (i, value) in f()?.borrow().iter().enumerate() {
            vec.push(A::alloc(&mut cs.ns(|| format!("alloc_{}", i)), || Ok(value))?);
        }
        Ok(vec)
    }

    fn alloc_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[I]>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let mut vec = vec![];
        for (i, value) in f()?.borrow().iter().enumerate() {
            vec.push(A::alloc_checked(&mut cs.ns(|| format!("alloc_checked_{}", i)), || {
                Ok(value)
            })?);
        }
        Ok(vec)
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[I]>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let mut vec = vec![];
        for (i, value) in f()?.borrow().iter().enumerate() {
            vec.push(A::alloc_input(&mut cs.ns(|| format!("alloc_input_{}", i)), || {
                Ok(value)
            })?);
        }
        Ok(vec)
    }

    fn alloc_input_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[I]>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let mut vec = vec![];
        for (i, value) in f()?.borrow().iter().enumerate() {
            vec.push(A::alloc_input_checked(
                &mut cs.ns(|| format!("alloc_input_checked_{}", i)),
                || Ok(value),
            )?);
        }
        Ok(vec)
    }
}

use core::num::NonZeroU32;
use rand_core::RngCore;

/// `OptionalRng` is a hack that is necessary because `Option<&mut R>` is not implicitly reborrowed
/// like `&mut R` is. This causes problems when a variable of type `Option<&mut R>`
/// is moved (eg, in a loop).
///
/// To overcome this, we define the wrapper `OptionalRng` here that can be borrowed
/// mutably, without fear of being moved.
pub struct OptionalRng<R>(pub Option<R>);

impl<R: RngCore> RngCore for OptionalRng<R> {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        (&mut self.0)
            .as_mut()
            .map(|r| r.next_u32())
            .expect("Rng was invoked in a non-hiding context")
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        (&mut self.0)
            .as_mut()
            .map(|r| r.next_u64())
            .expect("Rng was invoked in a non-hiding context")
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        (&mut self.0)
            .as_mut()
            .map(|r| r.fill_bytes(dest))
            .expect("Rng was invoked in a non-hiding context")
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        match &mut self.0 {
            Some(r) => r.try_fill_bytes(dest),
            None => Err(NonZeroU32::new(rand_core::Error::CUSTOM_START).unwrap().into()),
        }
    }
}

impl<R: RngCore> From<R> for OptionalRng<R> {
    fn from(other: R) -> Self {
        Self(Some(other))
    }
}

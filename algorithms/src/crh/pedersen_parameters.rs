use snarkos_models::curves::Group;

use rand::Rng;
use std::marker::PhantomData;

pub trait PedersenSize: Clone {
    const WINDOW_SIZE: usize;
    const NUM_WINDOWS: usize;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PedersenCRHParameters<G: Group, S: PedersenSize> {
    pub bases: Vec<Vec<G>>,
    _size: PhantomData<S>,
}

impl<G: Group, S: PedersenSize> PedersenCRHParameters<G, S> {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        let bases = (0..S::NUM_WINDOWS).map(|_| Self::base(S::WINDOW_SIZE, rng)).collect();
        Self {
            bases,
            _size: PhantomData,
        }
    }

    pub fn from(bases: Vec<Vec<G>>) -> Self {
        Self {
            bases,
            _size: PhantomData,
        }
    }

    fn base<R: Rng>(num_powers: usize, rng: &mut R) -> Vec<G> {
        let mut powers = vec![];
        let mut base = G::rand(rng);
        for _ in 0..num_powers {
            powers.push(base);
            base.double_in_place();
        }
        powers
    }
}

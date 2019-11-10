//! This is an interface for dealing with the kinds of
//! parallel computations involved in `snark`. It's
//! currently just a thin wrapper around `rayon`.
use rayon::{self, Scope};

#[derive(Copy, Clone)]
pub(crate) struct Worker {
    cpus: usize,
}

impl Worker {
    pub(crate) fn new() -> Worker {
        Self {
            cpus: rayon::current_num_threads(),
        }
    }

    pub(crate) fn log_num_cpus(&self) -> u32 {
        log2_floor(self.cpus)
    }

    pub(crate) fn scope<'a, F: 'a + Send + FnOnce(&Scope<'a>, usize) -> R, R: Send>(&self, elements: usize, f: F) -> R {
        let chunk_size = match elements < self.cpus {
            true => 1,
            false => elements / self.cpus,
        };
        rayon::scope(move |scope| f(scope, chunk_size))
    }
}

pub(crate) fn log2_floor(num: usize) -> u32 {
    assert!(num > 0);
    let mut pow = 0;
    while (1 << (pow + 1)) <= num {
        pow += 1;
    }
    pow
}

#[test]
fn test_log2_floor() {
    assert_eq!(log2_floor(1), 0);
    assert_eq!(log2_floor(2), 1);
    assert_eq!(log2_floor(3), 1);
    assert_eq!(log2_floor(4), 2);
    assert_eq!(log2_floor(5), 2);
    assert_eq!(log2_floor(6), 2);
    assert_eq!(log2_floor(7), 2);
    assert_eq!(log2_floor(8), 3);
}

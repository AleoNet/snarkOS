/// Returns log2
pub fn log2(x: usize) -> u32 {
    if x <= 1 {
        return 0;
    }

    let n = x.leading_zeros();
    core::mem::size_of::<usize>() as u32 * 8 - n - 1
}

/// Return ceil(x/y)
pub fn div_ceil(x: usize, y: usize) -> usize {
    (x + y - 1) / y
}

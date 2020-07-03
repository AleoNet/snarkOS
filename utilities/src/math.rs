/// Return ceil(x/y)
pub fn div_ceil(x: usize, y: usize) -> usize {
    (x + y - 1) / y
}

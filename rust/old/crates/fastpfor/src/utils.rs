
#[inline]
pub fn greatest_multiple(value: usize, factor: usize) -> usize {
    return value - value % factor;
}

use std::iter;
/// CircularBuffer is used to store the last elements of an endless sequence.
/// Oldest elements will be overwritten. The implementation focus on
/// speed. So memory allocations are avoided.
///
/// Usage example:
///```
/// extern crate tui_logger;
///
/// use tui_logger::CircularBuffer;
///
/// let mut cb : CircularBuffer<u64> = CircularBuffer::new(5);
/// cb.push(1);
/// cb.push(2);
/// cb.push(3);
/// cb.push(4);
/// cb.push(5);
/// cb.push(6); // This will overwrite the first element
///
/// // Total elements pushed into the buffer is 6.
/// assert_eq!(6,cb.total_elements());
///
/// // Thus the buffer has wrapped around.
/// assert_eq!(true,cb.has_wrapped());
///
/// /// Iterate through the elements:
/// {
///     let mut iter = cb.iter();
///     assert_eq!(Some(&2), iter.next());
///     assert_eq!(Some(&3), iter.next());
///     assert_eq!(Some(&4), iter.next());
///     assert_eq!(Some(&5), iter.next());
///     assert_eq!(Some(&6), iter.next());
///     assert_eq!(None, iter.next());
/// }
///
/// /// Iterate backwards through the elements:
/// {
///     let mut iter = cb.rev_iter();
///     assert_eq!(Some(&6), iter.next());
///     assert_eq!(Some(&5), iter.next());
///     assert_eq!(Some(&4), iter.next());
///     assert_eq!(Some(&3), iter.next());
///     assert_eq!(Some(&2), iter.next());
///     assert_eq!(None, iter.next());
/// }
///
/// // The elements in the buffer are now:
/// assert_eq!(vec![2,3,4,5,6],cb.take());
///
/// // After taking all elements, the buffer is empty.
/// let now_empty : Vec<u64> = vec![];
/// assert_eq!(now_empty,cb.take());
///```
pub struct CircularBuffer<T> {
    buffer: Vec<T>,
    next_write_pos: usize,
}
#[allow(dead_code)]
impl<T> CircularBuffer<T> {
    /// Create a new CircularBuffer, which can hold max_depth elements
    pub fn new(max_depth: usize) -> CircularBuffer<T> {
        CircularBuffer {
            buffer: Vec::with_capacity(max_depth),
            next_write_pos: 0,
        }
    }
    /// Return the number of elements present in the buffer
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    /// Push a new element into the buffer.
    /// Until the capacity is reached, elements are pushed.
    /// Afterwards the oldest elements will be overwritten.
    pub fn push(&mut self, elem: T) {
        let max_depth = self.buffer.capacity();
        if self.buffer.len() < max_depth {
            self.buffer.push(elem);
        } else {
            self.buffer[self.next_write_pos % max_depth] = elem;
        }
        self.next_write_pos += 1;
    }
    /// Take out all elements from the buffer, leaving an empty buffer behind
    pub fn take(&mut self) -> Vec<T> {
        let mut consumed = vec![];
        let max_depth = self.buffer.capacity();
        if self.buffer.len() < max_depth {
            consumed.append(&mut self.buffer);
        } else {
            let pos = self.next_write_pos % max_depth;
            let mut xvec = self.buffer.split_off(pos);
            consumed.append(&mut xvec);
            consumed.append(&mut self.buffer)
        }
        self.next_write_pos = 0;
        consumed
    }
    /// Total number of elements pushed into the buffer.
    pub fn total_elements(&self) -> usize {
        self.next_write_pos
    }
    /// If has_wrapped() is true, then elements have been overwritten
    pub fn has_wrapped(&self) -> bool {
        self.next_write_pos > self.buffer.capacity()
    }
    /// Return an iterator to step through all elements in the sequence,
    /// as these have been pushed (FIFO)
    pub fn iter(&mut self) -> iter::Chain<std::slice::Iter<T>, std::slice::Iter<T>> {
        let max_depth = self.buffer.capacity();
        if self.next_write_pos <= max_depth {
            // If buffer is not completely filled, then just iterate through it
            self.buffer.iter().chain(self.buffer[..0].iter())
        } else {
            let wrap = self.next_write_pos % max_depth;
            let it_end = self.buffer[..wrap].iter();
            let it_start = self.buffer[wrap..].iter();
            it_start.chain(it_end)
        }
    }
    /// Return an iterator to step through all elements in the reverse sequence,
    /// as these have been pushed (LIFO)
    pub fn rev_iter(
        &mut self,
    ) -> iter::Chain<std::iter::Rev<std::slice::Iter<T>>, std::iter::Rev<std::slice::Iter<T>>> {
        let max_depth = self.buffer.capacity();
        if self.next_write_pos <= max_depth {
            // If buffer is not completely filled, then just iterate through it
            self.buffer
                .iter()
                .rev()
                .chain(self.buffer[..0].iter().rev())
        } else {
            let wrap = self.next_write_pos % max_depth;
            let it_end = self.buffer[..wrap].iter().rev();
            let it_start = self.buffer[wrap..].iter().rev();
            it_end.chain(it_start)
        }
    }
}

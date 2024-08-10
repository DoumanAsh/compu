use core::{mem, slice};

///Utility Buffer
pub struct Buffer<const N: usize> {
    buffer: [mem::MaybeUninit<u8>; N],
    pub(crate) cursor: usize,
}

impl<const N: usize> Buffer<N> {
    ///Creates new instance
    pub const fn new() -> Self {
        debug_assert!(N >= 128, "Buffer less than 128 bytes makes no sense");
        Self {
            buffer: [mem::MaybeUninit::uninit(); N],
            cursor: 0,
        }
    }

    #[inline(always)]
    ///Returns split of buffer into written and unwritten parts
    pub fn split_buffer(&self) -> (&[u8], &[mem::MaybeUninit<u8>]) {
        debug_assert!(self.cursor <= self.buffer.len());

        unsafe {
            let (inited, uninit) = self.buffer.split_at_unchecked(self.cursor);
            (slice::from_raw_parts(inited.as_ptr() as _, self.cursor), uninit)
        }
    }

    #[inline(always)]
    ///Returns unconsumed data
    pub fn data(&self) -> &[u8] {
        self.split_buffer().0
    }

    #[inline(always)]
    ///Marks internal buffer as consumed fully
    pub fn consume(&mut self) {
        self.cursor = 0;
    }

    #[inline(always)]
    ///Returns spare capacity in buffer
    pub fn spare_capacity_mut(&mut self) -> &mut [mem::MaybeUninit<u8>] {
        debug_assert!(self.cursor <= self.buffer.len());

        unsafe { self.buffer.split_at_mut_unchecked(self.cursor).1 }
    }
}

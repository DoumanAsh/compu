//!Memory utilities alongside custom malloc implementation which uses Rust's allocator to satisfy common interface required by compression libraries

extern crate alloc;

use alloc::alloc::Layout;
pub use alloc::boxed::Box;
use core::{marker, mem, ptr, ops};
use core::ffi::{c_uint, c_void};

//Linux & win 32 bit are 8
#[cfg(not(any(target_os = "macos", all(windows, target_pointer_width = "64"))))]
const MIN_ALIGN: usize = 8;
//Mac and  win 64 bit are 16
#[cfg(any(target_os = "macos", all(windows, target_pointer_width = "64")))]
const MIN_ALIGN: usize = 16;

const LAYOUT_OFFSET: usize = mem::size_of::<usize>();

#[cold]
#[inline(never)]
fn unlikely_null() -> *mut c_void {
    ptr::null_mut()
}

#[inline]
///`malloc` impl with Rust allocator
pub unsafe extern "C" fn compu_malloc(size: usize) -> *mut c_void {
    if let Ok(layout) = Layout::from_size_align(size + LAYOUT_OFFSET, MIN_ALIGN) {
        let mem = alloc::alloc::alloc(layout);
        if !mem.is_null() {
            ptr::write(mem as *mut usize, size);
            return mem.add(LAYOUT_OFFSET) as _;
        }
    }

    unlikely_null()
}

#[inline]
///`free` impl with Rust allocator
pub unsafe extern "C" fn compu_free(mem: *mut c_void) {
    if !mem.is_null() {
        let mem = (mem as *mut u8).offset(-(LAYOUT_OFFSET as isize));
        let size = ptr::read(mem as *const usize);
        let layout = Layout::from_size_align_unchecked(size + LAYOUT_OFFSET, MIN_ALIGN);
        alloc::alloc::dealloc(mem, layout);
    }
}

#[allow(unused)]
///`malloc` alternative with Rust allocator
pub(crate) unsafe extern "C" fn compu_malloc_with_state(
    _: *mut c_void,
    size: usize,
) -> *mut c_void {
    compu_malloc(size)
}

#[allow(unused)]
///`alloc` alternative with Rust allocator
pub(crate) unsafe extern "C" fn compu_alloc(
    _: *mut c_void,
    items: c_uint,
    size: c_uint,
) -> *mut c_void {
    let size = match (items as usize).checked_mul(size as usize) {
        Some(0) | None => return unlikely_null(),
        Some(size) => size,
    };
    compu_malloc(size)
}

#[allow(unused)]
pub(crate) unsafe extern "C" fn compu_free_with_state(_: *mut c_void, mem: *mut c_void) {
    compu_free(mem)
}

#[cfg(feature = "brotli-rust")]
///Allocator implementation using Rust's global allocator
pub mod brotli_rust {
    extern crate alloc;

    use super::Box;
    use alloc::vec::Vec;

    ///Boxed slice wrapper
    pub struct BoxedSlice<T>(Box<[T]>);

    impl<T> Default for BoxedSlice<T> {
        #[inline(always)]
        fn default() -> Self {
            Self(Vec::new().into_boxed_slice())
        }
    }
    impl<T> brotli::SliceWrapper<T> for BoxedSlice<T> {
        #[inline(always)]
        fn slice(&self) -> &[T] {
            &self.0
        }
    }

    impl<T> brotli::SliceWrapperMut<T> for BoxedSlice<T> {
        #[inline(always)]
        fn slice_mut(&mut self) -> &mut [T] {
            &mut self.0
        }
    }

    #[derive(Copy, Clone, Default)]
    ///Default allocator
    pub struct BrotliAllocator;

    impl<T: Default> brotli::Allocator<T> for BrotliAllocator {
        type AllocatedMemory = BoxedSlice<T>;
        fn alloc_cell(&mut self, len: usize) -> Self::AllocatedMemory {
            let mut vec = Vec::with_capacity(len);
            for _ in 0..len {
                vec.push(Default::default());
            }
            BoxedSlice(vec.into_boxed_slice())
        }

        fn free_cell(&mut self, _data: Self::AllocatedMemory) {}
    }

    impl brotli::enc::BrotliAlloc for BrotliAllocator {}
}

///Minimal smart pointer implementation with custom dtor
///
///This allows to remove overhead of wrapping pointer into heap allocation for some codecs
///The restrictive API restricts possibility to make mistake but otherwise it is the same as using `Box<dyn T>`
///
///## Usage
///
///All codecs implement conversion into `Unique<dyn Encoder + Send + Sync` or `Unique<dyn Decoder + Send + Sync>`
///```rust
///use compu::encoder::{self, Encoder};
///use compu::decoder::{self, Decoder};
///let mut encoder: compu::mem::Unique<dyn Encoder + Send + Sync> = encoder::ZstdC::new(Default::default()).expect("to create zstd encoder").into();
///let mut decoder: compu::mem::Unique<dyn Decoder + Send + Sync> = decoder::ZstdC::new(Default::default()).expect("to create zstd decoder").into();
///
///encoder.reset();
///decoder.reset();
///```
pub struct Unique<T: ?Sized> {
    inner: ptr::NonNull<T>,
    dtor_fn: fn(ptr::NonNull<T>),
    //Based on rustc's unique impl
    //https://github.com/rust-lang/rust/blob/76c90957b7e422c4b9c45192b0197214d7de5a54/library/core/src/ptr/unique.rs#L42
    _marker: marker::PhantomData<T>,
}

impl<T: ?Sized> Unique<T> {
    #[inline(always)]
    pub(crate) fn new(inner: ptr::NonNull<T>, dtor_fn: fn(ptr::NonNull<T>)) -> Self {
        Self {
            inner,
            dtor_fn,
            _marker: marker::PhantomData,
        }
    }

    ///Creates instance from box
    pub fn from_box(value: Box<T>) -> Self {
        Self::new(Box::leak(value).into(), Self::dtor_boxed)
    }

    fn dtor_boxed(value: ptr::NonNull<T>) {
        let _ = unsafe {
            Box::from_non_null(value)
        };
    }
}

impl<T: ?Sized> ops::Deref for Unique<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe {
            self.inner.as_ref()
        }
    }
}

impl<T: ?Sized> ops::DerefMut for Unique<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.inner.as_mut()
        }
    }
}

unsafe impl<T: Send + ?Sized> Send for Unique<T> {
}
unsafe impl<T: Sync + ?Sized> Sync for Unique<T> {
}

impl<T: ?Sized> Drop for Unique<T> {
    #[inline(always)]
    fn drop(&mut self) {
        (self.dtor_fn)(self.inner)
    }
}

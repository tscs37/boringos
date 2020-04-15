use core::mem::size_of;
use core::marker::PhantomData;
use atomic::Atomic;

pub struct AtomicBitMap {
  data: *mut u8,
  size: usize,
}

impl AtomicBitMap {
  pub fn new(default: bool, size: usize) -> AtomicBitMap {
    use core::alloc::GlobalAlloc;
    let layout = alloc::alloc::Layout::array::<bool>(size)
      .expect("invalid size");
    let data = unsafe{crate::common::ALLOCATOR.alloc_zeroed(layout)};
    AtomicBitMap {
      data: data,
      size: size,
    }
  }
  pub fn set(offset: usize) {

  }
  pub fn get(offset: usize) {

  }
}

impl Drop for AtomicBitMap {
  fn drop(&mut self) {
    use core::alloc::GlobalAlloc;
    let layout = alloc::alloc::Layout::array::<bool>(self.size)
      .expect("invalid size");
    unsafe{crate::common::ALLOCATOR.dealloc(self.data, layout)}
  }
}
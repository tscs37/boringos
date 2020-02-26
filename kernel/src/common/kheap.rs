
use linked_list_allocator::Heap;
use spin::Mutex;
use alloc::alloc::{Alloc, GlobalAlloc, AllocErr, Layout};
use core::ops::Deref;
use core::ptr::NonNull;

use linked_list_allocator::LockedHeap;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();
/*
pub struct LockedHeap(Mutex<Heap>);

unsafe impl Alloc for LockedHeap {
  unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
    trace!("allocation: {:?}", layout);
    self.lock().alloc(layout)
  }
  unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {
    trace!("deallocation: {:?} / {:?}", ptr, layout);
    self.lock().dealloc(ptr, layout)
  }
}

unsafe impl GlobalAlloc for LockedHeap {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    self.lock().alloc(layout).ok().map_or(0 as *mut u8, |allocation| allocation.as_ptr())
  }
  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    self.lock().dealloc(NonNull::new_unchecked(ptr), layout)
  }
}

impl Deref for LockedHeap {
  type Target = Mutex<Heap>;

  fn deref(&self) -> &Mutex<Heap> {
    &self.0
  }
}

impl LockedHeap {
  pub unsafe fn init(&self, start: usize, size: usize) {
    self.lock().init(start, size);
  }
  pub const fn empty() -> LockedHeap {
    LockedHeap(Mutex::new(Heap::empty()))
  }
}*/
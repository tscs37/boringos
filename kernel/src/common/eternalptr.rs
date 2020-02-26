/// Provides a pointer that points to a pinned, eternal memory location
/// The object in this memory is never freed
/// Access is gated through an RW Mutex
/// 
/// This object is Send+Sync+Copy+Clone
/// 

use spin::RwLock;

pub struct EternalPtr<T> {
  _ptr: *mut EternalData<T>,
}

struct EternalData<T> {
  data: T,
  lock: RwLock,
}

impl<T> EternalPtr<T> {
  pub new(data: T) -> EternalPtr<T> {
    let layout = alloc::alloc::Layout::for_value(EternalData<T>);
    let allocation = alloc::alloc::alloc_zeroed(layout) as *mut T;
    core::mem::replace(allocation, data);
    drop(data);
  }
}
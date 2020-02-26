use core::sync::atomic::{AtomicBool, Ordering};
use core::ops::{Deref, DerefMut};
use core::cell::UnsafeCell;

pub struct KPut<T> {
  inner: UnsafeCell<T>,
  lock: UnsafeCell<AtomicBool>,
}

pub struct KPutGuard<'a, T> {
  inner: &'a mut T,
  lock: &'a mut AtomicBool,
}

unsafe impl<T> Sync for KPut<T> {}
unsafe impl<T> Send for KPut<T> {}
unsafe impl<'a, T> Send for KPutGuard<'a, T> {}

///
/// KPut is a special storage container; it only guards write access to the inner type T
/// and locks the write access to exclusivity
/// Contained types are responsible for ensuring reentrancy and atomicity in read operations
impl<T> KPut<T> {
  pub const fn new(t: T) -> Self {
    Self{
      inner: UnsafeCell::new(t),
      lock: UnsafeCell::new(AtomicBool::new(false)),
    }
  }
  pub fn read<'a>(&self) -> &'a T {
    // unsafe alias but inner type must guarantee atomicity
    return unsafe{&*self.inner.get()}
  }
  pub unsafe fn unsafe_write<'a>(&self) -> &'a mut T {
    return &mut *self.inner.get()
  }
  pub fn write<'a>(&self) -> KPutGuard<'a, T> {
    loop {
      if let Some(guard) = self.try_write() {
        return guard;
      }
    }
  }
  pub fn try_write<'a>(&self) -> Option<KPutGuard<'a, T>> {
    let lock: &AtomicBool= unsafe{&(*self.lock.get())};
    let lock_res = lock.compare_and_swap(false, true, Ordering::Relaxed);
    if lock_res { return None }
    return Some(KPutGuard{
      inner: unsafe{&mut *self.inner.get()},
      lock: unsafe{&mut *self.lock.get()},
    })
  }
}

impl<'a, T> Drop for KPutGuard<'a, T> {
  fn drop(&mut self) {
    self.lock.store(false, Ordering::Relaxed)
  }
}

impl<'a, T> Deref for KPutGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    return self.inner
  }
}

impl<'a, T> DerefMut for KPutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    return self.inner
  }
}
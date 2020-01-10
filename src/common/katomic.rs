
use core::ptr::NonNull;
use atomic::Atomic;
use atomic::Ordering;
use core::sync::atomic::AtomicU64;
use core::marker::PhantomData;

pub type NNTPtr<T> = NonNull<T>;
pub type NNPtr = NNTPtr<u8>;
pub type ATPtr<T> = Atomic<NNTPtr<T>>;
pub type APtr = ATPtr<u8>;

pub type OptAPtr = OptATPtr<u8>;
pub struct OptATPtr<T> {
  _type: PhantomData<T>,
  data: AtomicU64,
}

assert_eq_size!(check_nnptr; Option<NNPtr>, u64);
assert_eq_size!(check_opt_atomic_addr; OptAPtr, u64);
assert_eq_size!(check_opt_typed8_atomic_addr; OptATPtr<u8>, u64);
assert_eq_size!(check_opt_typed64_atomic_addr; OptATPtr<u64>, u64);
assert_eq_size!(check_opt_typed128_atomic_addr; OptATPtr<u128>, u64);

unsafe impl<T> Send for OptATPtr<T> {}
unsafe impl<T> Sync for OptATPtr<T> {}

impl<T> OptATPtr<T> {
  pub fn new(ptr: NNTPtr<T>) -> OptATPtr<T> {
    Self{
      _type: PhantomData{},
      data: AtomicU64::new(ptr.as_ptr() as u64),
    }
  }
  ///
  /// Swaps old for new value atomically, returns None if the swap failed
  pub fn cas(&self, old: NNTPtr<T>, new: NNTPtr<T>) -> Option<NNTPtr<T>> {
    NonNull::new(self.data.compare_exchange(
      old.as_ptr() as u64, new.as_ptr() as u64, 
      Ordering::SeqCst, Ordering::SeqCst).unwrap() as *mut T)
  }
  pub fn set(&self, addr: NNTPtr<T>) -> Option<NNTPtr<T>> {
    NonNull::new(self.data.swap(addr.as_ptr() as u64, Ordering::SeqCst) as *mut T)
  }
  pub fn get(&self) -> Option<NNTPtr<T>> {
    match self.data.load(Ordering::Relaxed) {
      0 => None,
      ptr => Some(unsafe{NonNull::new_unchecked(ptr as *mut T)}),
    }
  }
  pub const fn zero<Q>() -> OptATPtr<Q> {
    OptATPtr{
      _type: PhantomData{},
      data: AtomicU64::new(0)
    }
  }
  pub fn is_not_null(&self) -> bool {
    self.get().is_some()
  }
}

#[cfg(test)]
mod test {
  use super::*;
  #[test_case]
  fn check_optatptr_get_set() {
    let v: i64 = 42;
    let ptr_raw = &v as *const i64;
    let ptr = OptATPtr::new(NonNull::from(&v));
    assert_eq!(ptr_raw, ptr.get().expect("must return pointer").as_ptr(), 
      "ptr does not equal ptr get");
    let q: i64 = 32;
    ptr.set(NonNull::from(&q));
    assert_eq!(&q as *const i64, ptr.get().expect("must return pointer").as_ptr(), 
      "ptr set does not equal ptr get");
    assert!(ptr.is_not_null());
  }
}
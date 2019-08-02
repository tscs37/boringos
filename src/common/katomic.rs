
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
  ///
  /// Swaps old for new value atomically, returns None if the swap failed
  pub fn cas(&self, old: NNPtr, new: NNPtr) -> Option<NNPtr> {
    NonNull::new(self.data.compare_exchange(
      old.as_ptr() as u64, new.as_ptr() as u64, 
      Ordering::SeqCst, Ordering::SeqCst).unwrap() as *mut u8)
  }
  pub fn set(&self, addr: NNPtr) -> Option<NNPtr> {
    NonNull::new(self.data.swap(addr.as_ptr() as u64, Ordering::SeqCst) as *mut u8)
  }
  pub fn get(&self) -> Option<NNPtr> {
    match self.data.load(Ordering::Relaxed) {
      0 => None,
      ptr => Some(unsafe{NonNull::new_unchecked(ptr as *mut u8)}),
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
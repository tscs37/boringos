use crate::process_manager::Userspace;
use crate::vmem::PageManager;
use core::cell::Ref;

#[macro_use]
mod macros;
mod kinfo;

pub use crate::common::kinfo::*;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'static, Userspace> {
  use crate::USERSPACE;
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn pager<'a>() -> ::spin::MutexGuard<'a, PageManager> {
  let p = crate::PAGER.lock();
  p
}

pub fn alloc_page() -> Option<crate::vmem::PhysAddr> {
  unsafe { pager().alloc_page() }
}

pub fn release_page(pa: crate::vmem::PhysAddr) {
  unsafe { pager().free_page(pa) }
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn yield_to(t: u64) {
  let th = TaskHandle::from_c(t);
  debug!("Yielding to task {:?}", th);
  use crate::process_manager::TaskHandle;
  crate::userspace().in_scheduler_mut_spin(|mut sched| sched.yield_to(Some(th)));
}
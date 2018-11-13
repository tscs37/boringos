use ::process_manager::Userspace;
use ::vmem::PageManager;
use ::core::cell::Ref;

#[macro_use]
mod macros;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'static, Userspace> {
  use ::USERSPACE;
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn pager<'a>() -> ::spin::MutexGuard<'a, PageManager> {
  let p = ::PAGER.lock();
  p
}

pub fn alloc_page() -> Option<::vmem::PhysAddr> {
  unsafe { pager().alloc_page() }
}

pub fn release_page(pa: ::vmem::PhysAddr) {
  unsafe { pager().free_page(pa) }
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn yield_to(t: u64, p: u64) {
  let th = TaskHandle::from_c(t, p);
  debug!("Yielding to task {:?}", th);
  use ::process_manager::TaskHandle;
  ::userspace().in_scheduler_mut_spin(|mut sched| sched.yield_to(Some(th)));
}
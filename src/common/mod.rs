use crate::process_manager::{Userspace, Task, TaskHandle};
use crate::vmem::PageManager;
use core::cell::{Ref, RefMut};
use crate::vmem::PhysAddr;
use crate::vmem::pagelist::{PagePoolAllocationError, PagePoolReleaseError};

#[macro_use]
mod macros;
mod kinfo;

pub use crate::common::kinfo::*;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'a, Userspace> {
  use crate::USERSPACE;
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn kinfo<'a>() -> spin::RwLockReadGuard<'a, KernelInfo> {
  KERNEL_INFO.read()
}

pub fn kinfo_mut<'a>() -> spin::RwLockWriteGuard<'a, KernelInfo> {
  KERNEL_INFO.write()
}

pub fn with_current_task<T>(run: impl Fn(Option<Ref<Task>>) -> T) -> Result<T, ()> {
  let handle = current_taskhandle()?;
  userspace().in_scheduler(|sched| {
    let task = (sched).resolve_th(&handle);
    match task {
      None => run(None),
      Some(t) => run(Some(t.borrow()))
    }
  })
}

pub fn with_current_task_mut<T>(run: impl Fn(Option<RefMut<Task>>) -> T) -> Result<T, ()> {
  let handle = current_taskhandle()?;
  userspace().in_scheduler(|sched| {
    let task = (sched).resolve_th(&handle);
    match task {
      None => run(None),
      Some(t) => run(Some(t.borrow_mut()))
    }
  })
}

pub fn current_taskhandle() -> Result<TaskHandle, ()> {
  userspace().in_scheduler(|sched| {
    sched.current_task().clone()
  })
}

pub fn pager<'a>() -> ::spin::MutexGuard<'a, PageManager> {
  let p = crate::PAGER.lock();
  p
}

pub fn alloc_page() -> Result<PhysAddr, PagePoolAllocationError> {
  unsafe { pager().alloc_page() }
}

pub fn release_page(pa: PhysAddr) -> Result<(), PagePoolReleaseError>{
  unsafe { pager().free_page(pa) }
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn yield_to(t: u64) {
  let th = TaskHandle::from_c(t);
  debug!("Yielding to task {:?}", th);
  use crate::process_manager::TaskHandle;
  userspace().in_scheduler_mut_spin(|mut sched| sched.yield_to(Some(th)));
}
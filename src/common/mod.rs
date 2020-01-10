use crate::process_manager::{Userspace, Task, TaskHandle};
use crate::vmem::PageManager;
use core::cell::{Ref, RefMut};
use crate::vmem::pagelist::{PagePoolAllocationError, PagePoolReleaseError};

#[macro_use]
mod macros;
mod kinfo;
mod katomic;
mod kput;
mod kheap;
pub mod init;

pub use katomic::*;

pub use kheap::*;

pub use crate::common::kinfo::*;

pub use x86_64::{PhysAddr, VirtAddr};

pub use core::convert::TryInto;

pub use kput::*;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'a, Userspace> {
  use crate::USERSPACE;
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn kinfo<'a>() -> &'a KernelInfo {
  KERNEL_INFO.read()
}

pub fn kinfo_mut<'a>() -> KPutGuard<'a, KernelInfo> {
  KERNEL_INFO.try_write().expect("no lock on KINFO")
}

pub fn with_current_task<T>(run: impl Fn(Option<Ref<Task>>) -> T) -> Result<T, ()> {
  let handle = current_taskhandle()?;
  with_task(handle, run)
}

pub fn with_task<T>(th: TaskHandle, run: impl Fn(Option<Ref<Task>>) -> T) -> Result<T, ()> {
  userspace().in_scheduler(|sched| {
    let task = (sched).resolve_th(th);
    match task {
      None => run(None),
      Some(t) => run(Some(t.borrow()))
    }
  })
}

pub fn with_current_task_mut<T>(run: impl FnMut(Option<RefMut<Task>>) -> T) -> Result<T, ()> {
  let handle = current_taskhandle()?;
  with_task_mut(handle, run)
}

pub fn with_task_mut<T>(th: TaskHandle, mut run: impl FnMut(Option<RefMut<Task>>) -> T) -> Result<T, ()> {
  userspace().in_scheduler_mut(|sched| {
    let task = (sched).resolve_th(th);
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

pub fn pager() -> &'static PageManager {
  &crate::PAGER
}

pub fn alloc_page() -> Result<PhysAddr, PagePoolAllocationError> {
  unsafe { pager().alloc_page() }
}

pub fn release_page(pa: PhysAddr) -> Result<(), PagePoolReleaseError>{
  unsafe { pager().free_page(pa) }
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn yield_to(t: u128) {
  let th = TaskHandle::from_c(t);
  debug!("Yielding to task {}", th);
  let cur = userspace().in_scheduler_spin(|sched| sched.current_task());
  userspace().in_scheduler_mut_spin(|mut sched| sched.set_current_task(th));
  userspace().in_scheduler_spin(|sched| sched.yield_to(cur, Some(th)));
}
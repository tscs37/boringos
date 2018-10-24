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
pub extern "C" fn yield_to(th: usize, ph: usize) {
  let ytr: extern fn(usize, usize) = unsafe {
    let naked_yield: unsafe fn() = yield_to_raw;
    ::core::mem::transmute(naked_yield)
  };
  ytr(th, ph);
}

#[naked]
#[inline(never)]
#[cold]
pub unsafe fn yield_to_raw() {
  push_regs!();
  // rax = th
  // rbx = ph
  // rsp = old rsp value
  asm!(
    "
    mov rsp, $0
    push rax
    push rbx
    push rsp
    push $1
    ret
    " : 
    "=r"(::vmem::KSTACK_START):
    "r"(yield_stage1 as *mut u8 as u64):
    : "intel", "volatile"
  );
}

#[naked]
#[inline(never)]
#[cold]
pub unsafe fn yield_stage1() {
  let rsp: usize;
  let th_raw: usize;
  let ph_raw: usize;
  asm!(
    "
    pop $0
    pop $1
    pop $2
    ":
    "=r"(rsp),"=r"(th_raw),"=r"(ph_raw):

  );
  debug!("Entering Kernel Stack...");
  use ::process_manager::{ProcessHandle, TaskHandle, Handle};
  let ph = ProcessHandle::from(Handle::from(ph_raw as u64));
  let th = TaskHandle::from(ph, Handle::from(th_raw as u64));
  let us = ::userspace();
  {
    let s;
    loop {
      let sr = us.scheduler_mut();
      if sr.is_ok() {
        s = sr.unwrap();
        break;
      }
      debug!("scheduler unavailable, hlt'ing to wait");
      hlt_once!();
    }
    let treg = (*s).resolve_th(&th);
    {
      debug!("clearing out current task {}", s.current_task());
      let task = (*s).resolve_th(&s.current_task());
      match task {
        None => panic!("current task undefined in scheduler"),
        Some(task) => (*task).borrow_mut().save_and_clear(rsp),
      };
      debug!("task updating, yielding to scheduler");
    }
    s.yield_stage2(None) ;
  }
}


use ::{USERSPACE, PAGER};
use ::process_manager::Userspace;
use ::process_manager::TaskHandle;
use ::vmem::PageManager;
use ::core::cell::Ref;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'static, Userspace> {
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn pager<'a>() -> ::spin::MutexGuard<'a, PageManager> {
  unsafe { ::PAGER.force_unlock() };
  let p = ::PAGER.lock();
  unsafe { ::PAGER.force_unlock() };
  p
}

pub fn alloc_page() -> Option<::vmem::PhysAddr> {
  unsafe { pager().alloc_page() }
}

pub fn release_page(pa: ::vmem::PhysAddr) {
  unsafe { pager().free_page(pa) }
}

macro_rules! dump_stack_addr {
  () => { debug!("Stack at {:#018x}", stack_addr!()) }
}

macro_rules! stack_addr {
  () => { {
      let rsp: usize;
      unsafe { asm!("" : "={rsp}"(rsp)); };
      rsp
  } }
}

macro_rules! panic_on_drop {
  ($type_name:ident) => {
    impl Drop for $type_name {
      fn drop(&mut self) {
        panic!("Resource $type_name marked as panic_on_drop but was dropped");
      }
    }
  }
}

macro_rules! hlt_cpu {
  () => {
    loop {
      ::x86_64::instructions::hlt();
    }
  }
}

macro_rules! hlt_once {
  () => {
    ::x86_64::instructions::hlt();
  };
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
pub unsafe fn yield_to_raw() {
  let rsp: usize;
  let th_raw: usize;
  let ph_raw: usize;
  asm!(
    "
    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15
    " :
    "={rsp}"(rsp),"={rax}"(th_raw),"={rax}"(ph_raw):
    "{rsp}"(::vmem::KSTACK_START)
    :: "intel", "volatile"
  );
  //asm!(""::"{rsp}"(::vmem::KSTACK_START + 4096):::"intel", "volatile");
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
    unsafe { s.yield_stage2(None) };
  }
}

macro_rules! proc_yield {
  () => {
    ::common::yield_to(0,0);
    //::common::userspace().yield_to();
  }
}
macro_rules! ipc_call {
  ($fnc:ident, $data:expr) => {
    panic!("function ")
  }
}

macro_rules! ipc_return {
  ($data:expr) => { {
    let ipc_data = $data;
    debug!("Returning value {:#018x}", ipc_data);
    loop{}
  } }
}

macro_rules! ipc_error {
  ($code:expr) => { {
    let ipc_err = $code;
    debug!("Returning error {:#x}", ipc_err);
    loop{}
  } }
}

macro_rules! breakpoint {
  () => {
    ::x86_64::instructions::int3();
  };
}

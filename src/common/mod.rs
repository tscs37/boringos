
use ::USERSPACE;
use ::process_manager::Userspace;
use ::process_manager::TaskHandle;
use ::core::cell::Ref;

#[allow(dead_code)]

pub fn userspace<'a>() -> Ref<'static, Userspace> {
  unsafe {
    (*(USERSPACE.as_mut().expect("userspace required"))).borrow()
  }
}

pub fn yield_to(th: Option<TaskHandle>) {
  userspace().yield_to(th)
}

pub fn alloc_page() -> Option<::vmem::PhysAddr> {
  unsafe { ::PAGER.lock().alloc_page() }
}

pub fn release_page(pa: ::vmem::PhysAddr) {
  unsafe { ::PAGER.lock().free_page(pa) }
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

macro_rules! proc_yield {
  () => {
    ::common::userspace().yield_to(None);
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

/*macro_rules! pivot_to_kernel_stack {
  () => {
    {
      let old: usize = ::vmem::STACK_START; // start of kernel stack
      let new: usize = ::vmem::KSTACK_START; // new stack
      let old_rsp: usize;
      unsafe { asm!("" : "={rsp}"(old_rsp) : : : "intel", "volatile") };
      debug!("old: {:#018x}", old);
      debug!("new: {:#018x}", new);
      debug!("oldr:{:#018x}", old_rsp);
      let offset_rsp = old - old_rsp;
      debug!("ofsr:{:#018x}", offset_rsp);
      let new_rsp = new - offset_rsp;
      debug!("newr:{:#018x}", new_rsp);
      unsafe {::core::ptr::copy_nonoverlapping(
        old_rsp as *const u8,
        new_rsp as *mut u8,
        offset_rsp
        )};
      unsafe { asm!("" : : "{rsp}"(new_rsp) : : "intel", "volatile") };
    }
  }
}*/

macro_rules! pivot_to_kernel_stack {
  () => {
    let idx: u16 = 8; //::bindriver::cpu::gdt::SCHEDULER_IST_INDEX * 8;
    debug!("TSS IDX: {:#02x}", idx);
    dump_stack_addr!();
    unsafe {
      asm!(
        "
        ltr $0
        "::"{ax}"(idx):"ax":"intel", "volatile"
      )
    };
    dump_stack_addr!();
  };
}

macro_rules! breakpoint {
  () => {
    ::x86_64::instructions::int3();
  };
}

macro_rules! push_regs {
  () => {
    unsafe { asm!(
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
      pushfq
      "
      :::: "intel", "volatile"
    )};
  }
}

macro_rules! pop_regs {
  () => {
      unsafe { asm!(
      "
      popfq
      pop r15
      pop r14
      pop r13
      pop r12
      pop r11
      pop r10
      pop r9
      pop r8
      pop rbp
      pop rdi
      pop rsi
      pop rdx
      pop rcx
      pop rbx
      pop rax
      "
      :::: "intel", "volatile"
    )};
  }
}
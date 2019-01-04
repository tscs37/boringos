use core::cell::RefCell;
use crate::process_manager::memory::Memory;
use crate::vmem::PhysAddr;
use crate::process_manager::TaskHandle;

const DEFAULT_PAGE_LIMIT: usize = 1024;

#[derive(Debug, Clone)]
pub struct State {
  active: bool,
  mode: CPUMode,
  start_rip: PhysAddr,
  stack: Memory,
  memory: Memory,
  code: Memory,
  bss: Memory,
  rsp: usize,
  rbp: usize,
  page_limit: usize,
  signalrecv: usize, // Handle for Task Signals
  killh: usize, // Run this handler when we kill the task
  task_env: Option<Arc<crate::process_environment::TaskEnvironment>>, // If None, the task uses the parent's env, otherwise this one contains overwrites
}

fn null_fn() {
  panic!("entered null fn");
}

#[derive(Debug)]
pub enum StateError {
  ELFEntryZero,
  ELFBadExecutable,
  ELFExceedPHMax,
  ELFBadPH,
  ELFPHOverlap,
  ELFParseError(goblin::error::Error),
}

impl State {
  pub fn new_kernelstate(ptr: PhysAddr) -> State {
    trace!("new state with RIP: {}", ptr);
    let s = State {
      active: false,
      mode: CPUMode::Kernel,
      start_rip: ptr,
      stack: super::memory::Memory::new_stack(),
      memory: super::memory::Memory::new_usermemory(),
      bss: super::memory::Memory::new_staticmemory(),
      code: super::memory::Memory::new_codememory(),
      rsp: crate::vmem::STACK_START,
      rbp: crate::vmem::STACK_START,
      signalrecv: 0,
      killh: 0,
      task_env: None,
      page_limit: DEFAULT_PAGE_LIMIT,
    };
    s
  }
  pub fn new_elfstate(elf_ptr: &[u8]) -> Result<State, StateError> {
    use goblin::elf::Elf;
    let code_memory = super::memory::Memory::new_codememory();
    let bss_memory = super::memory::Memory::new_staticmemory();
    match Elf::parse(elf_ptr) {
      Ok(binary) => {
        if !binary.is_64
          || binary.is_lib
          || !binary.little_endian
          || binary.header.e_machine != 0x3E
        {
          return Err(StateError::ELFBadExecutable);
        }
        if binary.entry == 0 {
          return Err(StateError::ELFEntryZero);
        }
        trace!("ENTRY: {:#018x}", binary.entry);
        if binary.program_headers.len() > 32 {
          return Err(StateError::ELFExceedPHMax);
        }
        crate::kinfo_mut().mapping_task_image(Some(true));
        let old_code_memory = crate::kinfo_mut().set_memory_ref(&code_memory);
        let old_bss_memory = crate::kinfo_mut().set_memory_ref(&bss_memory);
        code_memory.map_rw();
        bss_memory.map();
        for ph in binary.program_headers {
          if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let filer = ph.file_range();
            let vmr = ph.vm_range();
            debug!("loading ELF section: {:#018x}", vmr.start);
            if ph.is_read() {
              // this is not according to ELF spec by a lot
              // but the linker is responsible here
              // this keeps the kernel simpler and prevents people from
              // effectively using ELF loading by being very annoying
              let base = filer.start + (&elf_ptr[0] as *const u8 as usize);
              trace!("base is {:#018x}, checking inmemory base...", base);
              let is_code_section = !ph.is_write() && vmr.start >= crate::vmem::CODE_START && vmr.end <= crate::vmem::CODE_END;
              let is_bss_section = ph.is_read() && vmr.start >= crate::vmem::BSS_START && vmr.end <= crate::vmem::BSS_END;
              if !is_code_section && !is_bss_section {
                panic!("bad section in ELF load: {:#018x}, RWX ({},{},{})", vmr.start, ph.is_read(), ph.is_write(), ph.is_executable());
              }
              trace!("PHFlags: R={}, W={}, X={}, {:#018x}, Code={}, BSS={}", ph.is_read(), ph.is_write(), ph.is_executable(), vmr.start, is_code_section, is_bss_section);
              let cur_real_base = {
                if is_code_section {
                  crate::kinfo().get_code_memory_ref_size()
                  * crate::vmem::PAGE_SIZE
                  + crate::vmem::CODE_START
                } else if is_bss_section {
                  crate::kinfo().get_bss_memory_ref_size()
                  * crate::vmem::PAGE_SIZE
                  + crate::vmem::BSS_START
                } else {
                  panic!("RWX on Program Header")
                }
              };
              if vmr.start > cur_real_base + crate::vmem::PAGE_SIZE {
                debug!("vmr.start > cur_real_base, setting zero page offset");
                let size = (vmr.start - cur_real_base) / crate::vmem::PAGE_SIZE;
                assert!(size < core::u16::MAX as usize, "size was {}, bigger than {}", size, core::u16::MAX);
                trace!("pretouching memory to zero page for program");
                crate::vmem::mapper::map_zero(PhysAddr::new_usize_or_abort(cur_real_base), size as u16);
                if is_code_section {
                  trace!("setting code memory offset");
                  code_memory.set_zero_page_offset(size as u16);
                  trace!("code_memory page_count = {}", code_memory.page_count());
                } else if is_bss_section {
                  trace!("setting bss memory offset");
                  bss_memory.set_zero_page_offset(size as u16);
                  trace!("bss_memory page_count = {}", bss_memory.page_count());
                } else {
                  panic!("offset on non-code memory");
                }
              }
              debug!(
                "PH, {:#018x} : {:#08x} ({:#08x}) -> {:#018x}",
                base,
                filer.start,
                filer.len(),
                vmr.start
              );
              unsafe {
                core::intrinsics::copy_nonoverlapping(
                  base as *const u8,
                  vmr.start as *mut u8,
                  vmr.len(),
                )
              };
            } else {
              return Err(StateError::ELFBadPH);
            }
            debug!("loaded ELF section: {:#018x}", vmr.start);
          }
        }
        code_memory.unmap();
        bss_memory.unmap();
        crate::kinfo_mut().mapping_task_image(Some(false));
        crate::kinfo_mut().set_memory_ref(&old_code_memory);
        crate::kinfo_mut().set_memory_ref(&old_bss_memory);
        drop(old_code_memory);
        drop(old_bss_memory);
        let s = State {
          active: false,
          mode: CPUMode::Kernel,
          start_rip: PhysAddr::new_or_abort(binary.entry),
          stack: super::memory::Memory::new_stack(),
          memory: super::memory::Memory::new_usermemory(),
          bss: bss_memory,
          code: code_memory,
          rsp: crate::vmem::STACK_START,
          rbp: crate::vmem::STACK_START,
          signalrecv: 0,
          killh: 0,
          task_env: None,
          page_limit: DEFAULT_PAGE_LIMIT,
        };
        Ok(s)
      }
      Err(e) => Err(StateError::ELFParseError(e)),
    }
  }
  pub fn new_nullstate() -> State {
    warn!("created nullstate");
    State {
      active: false,
      mode: CPUMode::Kernel,
      start_rip: PhysAddr::new(null_fn as u64).expect("null_fn must resolve"),
      stack: super::memory::Memory::new_nomemory(),
      memory: super::memory::Memory::new_nomemory(),
      bss: super::memory::Memory::new_nomemory(),
      code: super::memory::Memory::new_nomemory(),
      rsp: crate::vmem::STACK_START,
      rbp: crate::vmem::STACK_START,
      signalrecv: 0,
      killh: 0,
      task_env: None,
      page_limit: DEFAULT_PAGE_LIMIT,
    }
  }
  pub fn mode(&self) -> CPUMode {
    self.mode.clone()
  }
  pub fn activate(&mut self) {
    self.active = true;
  }
  pub fn map(&self) {
    trace!("mapping stack memory");
    self.stack.map();
    trace!("mapping user memory");
    self.memory.map();
    trace!("mapping code memory");
    self.code.map();
    trace!("mapping bss memory");
    self.bss.map();
  }
  pub fn unmap(&self) {
    trace!("unmapping stack memory");
    self.stack.unmap();
    trace!("unmapping user memory");
    self.memory.unmap();
    trace!("unmapping code memory");
    self.code.unmap();
    trace!("unmapping bss memory");
    self.bss.unmap();
  }
  pub fn rip(&self) -> u64 {
    self.start_rip.as_u64()
  }
  pub fn rsp(&self) -> u64 {
    self.rsp as u64
  }
  pub fn rbp(&self) -> u64 {
    self.rbp as u64
  }
  pub fn raise_page_limit(&mut self, pages: u16) -> u64 {
    self.page_limit += pages as usize;
    self.page_limit as u64
  }
  pub fn page_limit(&self) -> u64 {
    self.page_limit as u64
  }
  #[cold]
  #[inline(never)]
  pub fn switch_to(&mut self, next: &mut State) {
    debug!("Switching context");
    unsafe {
      self.active = false;
      next.active = true;
      self.stack.unmap();
      self.memory.unmap();
      next.stack.map();
      next.memory.map();
      debug!("Bye!");
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
      pushfq
      "
      :::: "intel", "volatile"
    );
      asm!("mov $0, rsp": "=r"(self.rsp) : : "memory": "intel", "volatile");
      asm!("mov rsp, $0": : "r"(next.rsp) : "memory" : "intel", "volatile");
      asm!("mov $0, rbp": "=r"(self.rbp) : : "memory": "intel", "volatile");
      asm!("mov rbp, $0": : "r"(next.rbp) : "memory" : "intel", "volatile");
      asm!(
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
      add rsp, 16
      "
      :::: "intel", "volatile"
    );
    }
  }
}

use alloc::sync::Arc;

pub unsafe fn switch_to(next_task: Arc<RefCell<crate::process_manager::Task>>, nt_handle: TaskHandle) -> ! {
  if next_task.borrow().state_is_null() {
    panic!("attempted to run null state");
  }
  {
    let next_task = next_task.borrow();
    let state = next_task.state();
    let kinfo = crate::kinfo_mut();
    let set_switching_tasks = kinfo.set_switching_tasks(false, true);
    let current_task_handle = kinfo.swap_current_task(0.into(), nt_handle);
    assert_eq!(set_switching_tasks, false, "enter userspace only outside task switching");
    assert_eq!(current_task_handle.into_c(), 0, "enter userspace from no running tasks");
    kinfo.set_memory_ref(&state.code);
    kinfo.set_memory_ref(&state.bss);
    kinfo.set_memory_ref(&state.stack);
    kinfo.set_memory_ref(&state.memory);
  }
  let rip = (next_task.borrow()).rip();
  let rsp = (next_task.borrow()).rsp();
  let rbp = (next_task.borrow()).rbp();
  let symrfp = crate::process_environment::symrf as *mut u8;
  trace!("symrfp at {:#018x}", symrfp as u64);
  trace!("mapping task memory");
  next_task.borrow().map();
  trace!("switch to task with rip = {:#018x}", rip);
  asm!(
    "
    mov rsp, $0
    mov rbp, $1
    add rsp, 32 # Adjust stack
    push rbx # Push symrfp argument (Symbol Resolver Function Pointer)
    mov rbx, 2
    push rbx # Push argc == 0 for unix start
    push rax
    ret
    "
    ::"r"(rsp), "r"(rbp), "{rax}"(rip), "{rbx}"(*symrfp)
    :"memory", "rbx", "rax": "intel", "volatile"
  );
  panic!("Returned somehow from non-cooperative switch-to");
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CPUMode {
  Kernel,
  Null, // Cannot run
  WASM, //TODO: convert to Interpreter(InterpreterHandle)
}

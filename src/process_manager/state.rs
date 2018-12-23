use core::cell::RefCell;
use crate::process_manager::memory::Memory;
use crate::vmem::PhysAddr;

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
    debug!("new state with RIP: {}", ptr);
    let s = State {
      active: false,
      mode: CPUMode::Kernel,
      start_rip: ptr,
      stack: super::memory::Memory::new_stack(),
      memory: super::memory::Memory::new_usermemory(),
      bss: super::memory::Memory::new_romemory(),
      code: super::memory::Memory::new_codememory(),
      rsp: crate::vmem::STACK_START,
      rbp: crate::vmem::STACK_START,
    };
    debug!("RIP={}", s.start_rip);
    s
  }
  pub fn new_elfstate(elf_ptr: &[u8]) -> Result<State, StateError> {
    use goblin::elf::Elf;
    let code_memory = super::memory::Memory::new_codememory();
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
        debug!("ENTRY: {:#018x}", binary.entry);
        if binary.program_headers.len() > 32 {
          return Err(StateError::ELFExceedPHMax);
        }
        crate::KERNEL_INFO.write().mapping_task_image(Some(true));
        let old_code_memory = crate::KERNEL_INFO.write().set_memory_ref(&code_memory);
        code_memory.map_rw();
        for ph in binary.program_headers {
          if ph.p_type == goblin::elf::program_header::PT_LOAD {
            let filer = ph.file_range();
            let vmr = ph.vm_range();
            if ph.is_read() {
              // this is not according to ELF spec by a lot
              // but the linker is responsible here
              // this keeps the kernel simpler and prevents people from
              // effectively using ELF loading by being very annoying
              let base = filer.start + (&elf_ptr[0] as *const u8 as usize);
              let cur_real_base = (crate::KERNEL_INFO.read().get_code_memory_ref_size() + 0)
                * crate::vmem::PAGE_SIZE
                + crate::vmem::CODE_START;
              if vmr.start > cur_real_base + crate::vmem::PAGE_SIZE {
                debug!("vmr.start({:#018x}) > cur_real_base({:#018x})", vmr.start, cur_real_base);
                let size = (vmr.start - cur_real_base) / crate::vmem::PAGE_SIZE - 1;
                debug!("pretouching memory to zero page for program");
                for x in 0..size {
                  crate::vmem::mapper::map_zero(PhysAddr::new_usize_or_abort(cur_real_base + crate::vmem::PAGE_SIZE * x));
                }
                assert!(size < core::u16::MAX as usize, "size was {}, bigger than {}", size, core::u16::MAX);
                code_memory.set_zero_page_offset(size as u16);
                debug!("code_memory page_count = {}", code_memory.page_count());
              } else if vmr.start < cur_real_base {
                error!("vmr.start {} < cur_real_base {}", vmr.start, cur_real_base);
                return Err(StateError::ELFPHOverlap);
              }
              debug!(
                "PH, {:#018x} : {:#08x} ({:#08x}) -> {:#018x} ({:#018x})",
                base,
                filer.start,
                filer.len(),
                vmr.start,
                vmr.len()
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
          }
        }
        crate::KERNEL_INFO
          .write()
          .set_code_memory_ref(old_code_memory);
        crate::KERNEL_INFO.write().mapping_task_image(Some(false));
        code_memory.unmap();
        let s = State {
          active: false,
          mode: CPUMode::Kernel,
          start_rip: PhysAddr::new_or_abort(binary.entry),
          stack: super::memory::Memory::new_stack(),
          memory: super::memory::Memory::new_usermemory(),
          bss: super::memory::Memory::new_romemory(),
          code: code_memory,
          rsp: crate::vmem::STACK_START,
          rbp: crate::vmem::STACK_START,
        };
        debug!("RIP={}", s.start_rip);
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
    }
  }
  pub fn mode(&self) -> CPUMode {
    self.mode.clone()
  }
  pub fn activate(&mut self) {
    self.active = true;
  }
  pub fn map(&self) {
    debug!("mapping stack memory");
    self.stack.map();
    debug!("mapping user memory");
    self.memory.map();
    debug!("mapping code memory");
    self.code.map();
    debug!("mapping bss memory");
    self.bss.map();
  }
  pub fn unmap(&self) {
    debug!("unmapping stack memory");
    self.stack.unmap();
    debug!("unmapping user memory");
    self.memory.unmap();
    debug!("unmapping code memory");
    self.code.unmap();
    debug!("unmapping bss memory");
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

pub unsafe fn switch_to(next_task: Arc<RefCell<crate::process_manager::Task>>) -> ! {
  if next_task.borrow().state_is_null() {
    panic!("attempted to run null state");
  }
  {
    let next_task = next_task.borrow();
    let state = next_task.state();
    let kinfo =crate::KERNEL_INFO.write();
    kinfo.set_memory_ref(&state.code);
    kinfo.set_memory_ref(&state.bss);
    kinfo.set_memory_ref(&state.stack);
    kinfo.set_memory_ref(&state.memory);
  }
  let rip = (next_task.borrow()).rip();
  let rsp = (next_task.borrow()).rsp();
  let rbp = (next_task.borrow()).rbp();
  let symrfp = 0;
  debug!("manual switch to rip = {:#018x}", rip);
  next_task.borrow().map();
  debug!("mapped memory, switching stack and clearing registers");
  asm!(
    "
    mov rsp, $0
    mov rbp, $1
    add rsp, 24 # Adjust stack
    push rbx # Push symrfp argument (Symbol Resolver Function Pointer)
    push 0   # Push argc == 0 for unix start
    push rax
    ret
    "
    ::"r"(rsp), "r"(rbp), "{rax}"(rip), "{rbx}"(symrfp)
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

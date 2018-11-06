use ::vmem::PhysAddr;

#[derive(Clone)]
pub struct State {
  active: bool,
  mode: CPUMode,
  instr_ptr: PhysAddr,
  stack: ::process_manager::memory::Stack,
  memory: ::process_manager::memory::Memory,
  rsp: usize,
  rbp: usize,
}

fn null_fn() { panic!("entered null fn"); }

impl State {
  pub fn new_kernelstate(ptr: fn()) -> State {
    State{
      active: false,
      mode: CPUMode::Kernel,
      instr_ptr: PhysAddr::new(ptr as *mut u8 as u64).expect("kernelstate needs function pointer"),
      stack: super::memory::Stack::new_userstack(),
      memory: super::memory::Memory::new_usermemory(),
      rsp: ::vmem::STACK_START,
      rbp: ::vmem::STACK_START,
    }
  }
  pub fn new_nullstate() -> State {
    State{
      active: false,
      mode: CPUMode::Kernel,
      instr_ptr: PhysAddr::new(null_fn as u64).expect("null_fn must resolve"),
      stack: super::memory::Stack::new_nostack(),
      memory: super::memory::Memory::new_nomemory(),
      rsp: ::vmem::STACK_START,
      rbp: ::vmem::STACK_START,
    }
  }
  #[cold]
  #[inline(never)]
  pub fn switch_to(&mut self, next: &mut State) {
    self.active = false;
    next.active = true;
    self.stack.unmap();
    next.stack.map();
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
    asm!("mov rbp, $0": : "r"(next.rsp) : "memory" : "intel", "volatile");
    pop_regs!();
    unsafe { asm!("ret") };
  }
}

#[derive(Clone)]
pub enum CPUMode {
  Kernel,
  WASM //TODO: convert to Interpreter(InterpreterHandle)
}

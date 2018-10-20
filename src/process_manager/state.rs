use ::vmem::PhysAddr;
use ::alloc::boxed::Box;

pub struct State {
  mode: CPUMode,
  instr_ptr: PhysAddr,
  stack: ::process_manager::stack::Stack,
  data_registers: Box<Registers>,
}

impl State {
  pub fn new_kernelstate(ptr: fn()) -> State {
    State{
      mode: CPUMode::Kernel,
      instr_ptr: PhysAddr::new(ptr as *mut u8 as u64).expect("kernelstate needs function pointer"),
      stack: super::stack::Stack::new_64kstack(),
      data_registers: Box::new(Registers::new()),
    }
  }
  pub fn restore(&mut self) {
    debug!("mapping stack of new process");
    self.stack.map();
    debug!("loading RIP and RSP");
    let rip = self.instr_ptr.as_usize();
    let regs = self.data_registers.as_ref();
    debug!("prepare to yield");
    unsafe { asm!(
      "
      push $1
      ret
      "
       : : "{rsp}"(regs.rsp), "r"(rip) : 
       "rsp") }
  }
  #[inline(never)]
  #[naked]
  pub fn save_and_clear(&mut self) {
    panic!("todo: state save_and_clear")
  }
}

pub struct Registers {
  rax: u64, rbx: u64, rcx: u64, rdx: u64,
  rsi: u64, rdi: u64,
  rbp: u64, rsp: u64,
  r8: u64, r9: u64, r10: u64, r11: u64,
  r12: u64, r13: u64, r14: u64, r15: u64,
  rflags: u64,
}

impl Registers {
  fn new() -> Self {
    Registers{
      rax: 0, rbx: 0, rcx: 0, rdx: 0,
      rsi: 0, rdi: 0,
      rbp: 0, rsp: ::vmem::STACK_START as u64,
      r8: 0, r9: 0, r10: 0, r11: 0, 
      r12: 0, r13: 0, r14: 0, r15: 0,
      rflags: 0,
    }
  }
}

pub enum CPUMode {
  Kernel,
  WASM //TODO: convert to Interpreter(InterpreterHandle)
}

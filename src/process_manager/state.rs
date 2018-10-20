use ::vmem::PhysAddr;
use ::alloc::boxed::Box;

pub struct State {
  mode: CPUMode,
  instr_ptr: PhysAddr,
  stack: ::process_manager::stack::Stack,
  rsp: usize,
}

impl State {
  pub fn new_kernelstate(ptr: fn()) -> State {
    State{
      mode: CPUMode::Kernel,
      instr_ptr: PhysAddr::new(ptr as *mut u8 as u64).expect("kernelstate needs function pointer"),
      stack: super::stack::Stack::new_64kstack(),
      rsp: ::vmem::STACK_START,
    }
  }
  pub fn restore(&mut self) {
    debug!("mapping stack of new process");
    self.stack.map();
    debug!("loading RIP and RSP");
    let rip = self.instr_ptr.as_usize();
    debug!("prepare to yield");
    unsafe { asm!(
      "
      push $1
      ret
      "
       : : "{rsp}"(self.rsp), "r"(rip) : 
       "rsp") }
  }
  #[inline(never)]
  #[naked]
  pub fn save_and_clear(&mut self) {
    unsafe { asm!(
      "
      push rax
      push rbx
      push rcx
      push rdx
      push rsi
      push rdi
      push r8
      push r9
      push r10
      push r11
      push r12
      push r13
      push r14
      push r15
      push rflags
      "
      :"={rsp}"(self.rsp)::: "intel", "volatile"
    )};
    self.stack.unmap();
    panic!("todo: state save_and_clear")
  }
}

pub enum CPUMode {
  Kernel,
  WASM //TODO: convert to Interpreter(InterpreterHandle)
}

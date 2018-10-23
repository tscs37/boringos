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
    debug!("mapping stack");
    self.stack.map();
    //TODO: switch stack
    error!("did not switch stack");
    debug!("restoring stack and returning");
    pop_regs!();
    unsafe { asm!("ret") };
  }
  pub fn restore_new(&mut self) {
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
  pub fn save_and_clear(&mut self, rsp: usize) {
    self.stack.unmap();
    self.rsp = rsp;
    panic!("todo: state save_and_clear")
  }
}

pub enum CPUMode {
  Kernel,
  WASM //TODO: convert to Interpreter(InterpreterHandle)
}

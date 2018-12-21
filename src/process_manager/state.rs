use ::vmem::PhysAddr;

pub struct State {
  active: bool,
  mode: CPUMode,
  start_rip: PhysAddr,
  stack: crate::process_manager::memory::Stack,
  memory: ::process_manager::memory::Memory,
  rsp: usize,
  rbp: usize,
}

fn null_fn() { panic!("entered null fn"); }

impl Clone for State {
  fn clone(&self) -> Self {
    State{
      active: false,
      mode: self.mode,
      start_rip: self.start_rip,
      stack: self.stack,
      memory: self.memory,
      rsp: self.rsp,
      rbp: self.rbp,
    }
  }
}

impl State {
  pub fn new_kernelstate(ptr: fn()) -> State {
    debug!("new state with RIP: {:#018x}", ptr as usize);
    let s = State{
      active: false,
      mode: CPUMode::Kernel,
      start_rip: PhysAddr::new(ptr as u64).expect("kernelstate needs function pointer"),
      stack: super::memory::Stack::new_userstack(),
      memory: super::memory::Memory::new_usermemory(),
      rsp: ::vmem::STACK_START,
      rbp: ::vmem::STACK_START,
    };
    debug!("RIP={}", s.start_rip);
    s
  }
  pub fn new_nullstate() -> State {
    warn!("created nullstate");
    State{
      active: false,
      mode: CPUMode::Kernel,
      start_rip: PhysAddr::new(null_fn as u64).expect("null_fn must resolve"),
      stack: super::memory::Stack::new_nostack(),
      memory: super::memory::Memory::new_nomemory(),
      rsp: ::vmem::STACK_START,
      rbp: ::vmem::STACK_START,
    }
  }
  pub fn activate(&mut self) {
    self.active = true;
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

pub unsafe fn switch_to(next_task: *mut super::Task) -> ! {
  let next = (*next_task).get_state_and_activate();
  if next.mode == CPUMode::Null {
    panic!("attempted to run null state");
  }
  debug!("manual switch to rip = {}", next.start_rip);
  next.stack.map();
  next.memory.map();
  debug!("mapped memory, switching stack and clearing registers");
  asm!("mov rsp, $0": : "r"(next.rsp) : "memory" : "intel", "volatile");
  asm!("mov rbp, $0": : "r"(next.rbp) : "memory" : "intel", "volatile");
  asm!(
    "
    mov rsp, $0
    mov rbp, $1
    push $0
    mov r15, 0
    mov r14, 0
    mov r13, 0
    mov r12, 0
    mov r11, 0
    mov r10, 0
    mov r9, 0
    mov r8, 0
    mov rdi, 0
    mov rsi, 0
    mov rdx, 0
    mov rcx, 0
    mov rbx, 0
    mov rax, 0
    pop rax
    ret
    "
    ::"r"(next.rsp), "r"(next.rbp), "r"(next.start_rip)
    :"memory", "rbp", "rsp", "rax": "intel", "volatile"
  );
  panic!("Returned somehow from non-cooperative switch-to");
}

#[derive(Clone, PartialEq)]
pub enum CPUMode {
  Kernel,
  Null, // Cannot run
  WASM //TODO: convert to Interpreter(InterpreterHandle)
}

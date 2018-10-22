
use ::process_manager::{Task,Process};
use ::core::ptr::NonNull;
use ::alloc::collections::BTreeMap;
use ::core::cell::RefCell;
use ::alloc::rc::Rc;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Handle(u64);

impl Handle {
  pub const fn into(self) -> u64 {
    self.0
  }
  pub const fn from(x: u64) -> Handle {
    Handle(x)
  }
  pub fn gen() -> Handle {
    if !::bindriver::cpu::has_rdrand() {
      panic!("BoringOS requires a CPU with RDRAND Support");
    }
    let rnd: u64;
    let retry: u32;
    unsafe{
      asm!(
        "
        mov ecx, 100
        retry_handle_gen:
          rdrand rax
          jnc .done_handle_gen
          cmp ecx, 0
          jz .done_handle_gen
          loop retry_handle_gen
        .done_handle_gen:
        ":
        "={rax}"(rnd), "={ecx}"(retry)::"rax", "ecx":"intel", "volatile"
      );
    }
    if retry < 0 { panic!("could not get random number")}
    Handle(rnd)
  }
}

impl ::core::fmt::Display for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:016x}", self.0))
  }
}

#[derive(Clone)]
pub struct ProcessHandleRegistry(BTreeMap<ProcessHandle, 
  Rc<RefCell<Process>>>);

impl ProcessHandleRegistry {
  pub fn new() -> ProcessHandleRegistry {
    ProcessHandleRegistry(BTreeMap::new())
  }
  pub fn insert(&mut self, ph: &ProcessHandle, p: Rc<RefCell<Process>>) {
    self.0.insert(*ph, p);
  }
  pub fn resolve(&self, ph: &ProcessHandle) -> Option<&Rc<RefCell<Process>>> {
    self.0.get(ph)
  }
  pub fn resolve_task(&self, th: &TaskHandle) -> Option<&Rc<RefCell<Process>>> {
    self.resolve(&th.process_handle())
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ProcessHandle(Handle);

impl ProcessHandle {
  pub const fn into(self) -> Handle {
    self.0
  }
  pub const fn from(x: Handle) -> Self {
    ProcessHandle(x)
  }
  pub fn gen() -> ProcessHandle {
    ProcessHandle(Handle::gen())
  }
}

impl ::core::fmt::Display for ProcessHandle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{}", self.0))
  }
}

#[derive(Clone)]
pub struct TaskHandleRegistry(BTreeMap<TaskHandle, 
  Rc<RefCell<Task>>>);


impl TaskHandleRegistry {
  pub fn new() -> TaskHandleRegistry {
    TaskHandleRegistry(BTreeMap::new())
  }
  pub fn insert(&mut self, th: &TaskHandle, t: Rc<RefCell<Task>>) {
    self.0.insert(*th, t);
  }
  pub fn resolve(&self, th: &TaskHandle) -> Option<&Rc<RefCell<Task>>> {
    self.0.get(th)
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskHandle(Handle, ProcessHandle);

impl TaskHandle {
  pub fn process_handle(&self) -> ProcessHandle {
    self.1
  }
  pub const fn into(self) -> Handle {
    self.0
  }
  pub const fn from(p: ProcessHandle, x: Handle) -> Self {
    TaskHandle(x, p)
  }
  pub fn gen(p: ProcessHandle) -> TaskHandle {
    TaskHandle(Handle::gen(), p)
  }
}

impl ::core::fmt::Display for TaskHandle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{}:{}", self.0, self.process_handle()))
  }
}
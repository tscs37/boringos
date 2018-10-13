
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
}

impl ::core::fmt::Display for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{}", self.0))
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
}

impl ::core::fmt::Display for TaskHandle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{}:{}", self.0, self.process_handle()))
  }
}
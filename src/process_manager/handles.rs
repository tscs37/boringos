
use crate::process_manager::Task;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::cell::RefCell;

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
    Handle(crate::bindriver::cpu::rng::get_u64())
  }
}

impl ::core::fmt::Display for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:x}", self.0))
  }
}

impl ::core::fmt::LowerHex for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:016x}", self.0))
  }
}

impl ::core::fmt::UpperHex for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:016X}", self.0))
  }
}

#[derive(Clone)]
pub struct TaskHandleRegistry(BTreeMap<TaskHandle, 
  Arc<RefCell<Task>>>);


impl TaskHandleRegistry {
  pub fn new() -> TaskHandleRegistry {
    TaskHandleRegistry(BTreeMap::new())
  }
  pub fn insert(&mut self, th: &TaskHandle, t: Task) {
    self.0.insert(*th, Arc::new(RefCell::new(t)));
  }
  pub fn resolve(&self, th: &TaskHandle) -> Option<&Arc<RefCell<Task>>> {
    self.0.get(th)
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskHandle(Handle);

impl TaskHandle {
  pub const fn into(self) -> Handle {
    self.0
  }
  pub const fn into_c(self) -> u64 {
    self.0.into()
  }
  pub const fn from(x: Handle) -> Self {
    TaskHandle(x)
  }
  pub const fn from_c(t: u64) -> Self {
    TaskHandle(Handle(t))
  }
  pub fn gen() -> TaskHandle {
    TaskHandle(Handle::gen())
  }
  pub fn is_scheduler(&self) -> bool {
    self.0.into() == 0
  }
  pub fn zero() -> TaskHandle {
    Self::from(Handle(0))
  }
}

impl From<u64> for TaskHandle {
  fn from(t: u64) -> Self {
    TaskHandle(Handle::from(t))
  }
}

impl ::core::fmt::Display for TaskHandle {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{}", self.0))
  }
}

impl ::core::fmt::LowerHex for TaskHandle {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:x}", self.0))
  }
}

impl ::core::fmt::UpperHex for TaskHandle {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:X}", self.0))
  }
}

assert_eq_size!(check_task_handle_size; TaskHandle, u64);
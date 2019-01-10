
use crate::process_manager::Task;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::cell::RefCell;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Handle(u128);

impl Handle {
  pub fn gen() -> Handle {
    Handle(crate::bindriver::cpu::rng::get_u128())
  }
}

impl From<u128> for Handle {
  fn from(t: u128) -> Handle {
    Handle(t)
  }
}

impl Into<u128> for Handle {
  fn into(self) -> u128 {
    self.0
  }
}

impl Into<[u8; 16]> for Handle {
  fn into(self) -> [u8; 16] {
    self.0.to_ne_bytes()
  }
}

impl ::core::fmt::Display for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:x}", self))
  }
}

impl ::core::fmt::Debug for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:x}", self))
  }
}

impl ::core::fmt::LowerHex for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:032x}", self.0))
  }
}

impl ::core::fmt::UpperHex for Handle {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:032X}", self.0))
  }
}

#[derive(Clone)]
pub struct TaskHandleRegistry(BTreeMap<TaskHandle, 
  Arc<RefCell<Task>>>);


impl TaskHandleRegistry {
  pub fn new() -> TaskHandleRegistry {
    TaskHandleRegistry(BTreeMap::new())
  }
  pub fn insert(&mut self, th: TaskHandle, t: Task) {
    self.0.insert(th, Arc::new(RefCell::new(t)));
  }
  pub fn resolve(&self, th: TaskHandle) -> Option<&Arc<RefCell<Task>>> {
    self.0.get(&th)
  }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct TaskHandle(Handle);

impl TaskHandle {
  pub const fn into(self) -> Handle {
    self.0
  }
  pub const fn into_c(self) -> u128 {
    (self.0).0
  }
  pub const fn from(x: Handle) -> Self {
    TaskHandle(x)
  }
  pub const fn from_c(t: u128) -> Self {
    TaskHandle(Handle(t))
  }
  pub fn gen() -> TaskHandle {
    TaskHandle(Handle::gen())
  }
  pub fn is_scheduler(&self) -> bool {
    let t : u128 = self.0.into();
    t == 0
  }
  pub fn zero() -> TaskHandle {
    Self::from(Handle(0))
  }
}

impl From<u128> for TaskHandle {
  fn from(t: u128) -> Self {
    TaskHandle(Handle::from(t))
  }
}

impl Into<u128> for TaskHandle {
  fn into(self) -> u128 {
    self.into_c()
  }
}

impl From<[u8; 16]> for TaskHandle {
  fn from(x: [u8; 16]) -> TaskHandle {
    TaskHandle(Handle(u128::from_le_bytes(x)))
  }
}

impl ::core::fmt::Display for TaskHandle {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:?}", self.0))
  }
}

impl ::core::fmt::Debug for TaskHandle {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
      f.write_fmt(format_args!("{:?}", self.0))
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

assert_eq_size!(check_task_handle_size; TaskHandle, u128);
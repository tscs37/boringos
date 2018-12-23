
use alloc::string::String;
//use crate::process_manager::{State};
use alloc::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct IPCName(String);

#[derive(Clone)]
pub enum IPCTarget{
  //System(State),
  //TODO: add task IPC target
}

#[derive(Clone)]
pub struct IPCRegistry(BTreeMap<IPCName, IPCTarget>);

pub fn init() {

}

pub extern fn symrf(sym_type: u16, sym_name: *mut u8, sym_name_size: u8) -> *mut u8 {
  panic!("ffi not allowed yet")
}
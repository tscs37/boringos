
use ::alloc::string::String;
use ::process_manager::{TaskHandle, State};
use alloc::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct IPCName(String);

#[derive(Clone)]
pub enum IPCTarget{
  System(State),
  //TODO: add task IPC target
}

#[derive(Clone)]
pub struct IPCRegistry(BTreeMap<IPCName, IPCTarget>);

pub fn init() {

}
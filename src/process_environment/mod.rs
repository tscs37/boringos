
use alloc::string::String;
//use crate::process_manager::{State};
use alloc::collections::BTreeMap;
use crate::process_manager::TaskHandle;
use symrfp::SymbolType;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Visibility {
  Full,
  Hidden,
  AccessDeny,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct IPCName(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskData(String);

#[derive(Clone)]
pub enum IPCTarget{
  //System(State),
  //TODO: add task IPC target
}

#[derive(Clone)]
pub struct IPCRegistry(BTreeMap<IPCName, (Visibility, IPCTarget)>);

pub struct TaskDataRegistry(BTreeMap<(TaskHandle, String), (Visibility, TaskData)>);

impl TaskDataRegistry {
  pub fn add(name: String, vis: Visibility, data: TaskData) { panic!("TODO:") }
  pub fn set(name: String, vis: Visibility, data: TaskData) { panic!("TODO:") }
  pub fn del(name: String) -> (Visibility, TaskData) { panic!("TODO:") }
  pub fn get(name: String) -> (Visibility, TaskData) { panic!("TODO:") }
}

pub fn init() {

}

pub extern fn symrf(sym_type: u16, sym_name: &str) -> *mut u8 {
  let st = SymbolType::from(sym_type);
  match st {
    Some(st) => {
      match st {
        SymbolType::TestSymbolResolver => {
          return 42 as *mut u8;
        }
        _ => { panic!("ffi not allowed yet") }
      }
    }
    None => return 0 as *mut u8,
  }
}
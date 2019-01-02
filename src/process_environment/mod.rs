
use alloc::string::String;
//use crate::process_manager::{State};
use alloc::collections::BTreeMap;
use crate::process_manager::TaskHandle;
use symrfp::SymbolType;
use spin::RwLock;

mod kcalls;

// The task environment provides a set of data the current task has set
// or available to it (set by other tasks)
// The kernel inherits task data; if the symbol is not found for the current task,
// it looks at the task env of the parent up until the highest available parent
// is reached. Symbol lookups should be rare as a process only needs to do them
// when it first accesses a function. To avoid frequent lookups, a task should
// cache them and if spawning subtasks (threads), they should have access to this
// cache as well.
#[derive(Debug)]
pub struct TaskEnvironment {
  ipc_registry: IPCRegistry,
  task_data_registry: TaskDataRegistry,
  // TaskEnv locks internally, externally all operations must be atomic
  //TODO:
  _rw_lock: RwLock<()>,
}

unsafe impl Sync for TaskEnvironment{}

impl TaskEnvironment {
  pub fn taskdata_add() {}
  pub fn taskdata_set() {}
  pub fn taskdata_get() {}
  pub fn taskdata_del() {}
  pub fn ipc_add() {}
  // ipc_proxy replaces an IPC symbol with the specified symbol and returns the pointer of the previous value
  // ipc does not allow removing symbols, programs may hold a pointer to it
  // proxy is how you change it; you get the old pointer and set your own as the new
  // you can forward the call, modify parameters and return data or simply do something
  // else entirely
  pub fn ipc_proxy() {}
  pub fn ipc_get() {}
}

impl TaskEnvironment {
  pub fn new() -> Self {
    TaskEnvironment {
      _rw_lock: RwLock::new(()),
      ipc_registry: IPCRegistry::new(),
      task_data_registry: TaskDataRegistry::new(),
    }
  }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Visibility {
  Full, // Allow listing the entry
  Hidden, // Do not list entry but available via _get() (sold under the counter)
  Private(TaskHandle), // Only the specified Task may access this item
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct IPCName(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskData(String);

#[derive(Clone, Debug)]
pub enum IPCTarget{
  //System(State),
  //TODO: add task IPC target
}

#[derive(Clone, Debug)]
pub struct IPCRegistry(BTreeMap<IPCName, (Visibility, IPCTarget)>);

impl IPCRegistry {
  fn new() -> Self {
    IPCRegistry(BTreeMap::new())
  }
}

#[derive(Clone, Debug)]
pub struct TaskDataRegistry(BTreeMap<(TaskHandle, String), (Visibility, TaskData)>);

impl TaskDataRegistry {
  fn new() -> Self {
    TaskDataRegistry(BTreeMap::new())
  }
}

pub fn init() {
  crate::kinfo_mut().init_task_env();
}

pub extern fn symrf(sym_type: u16, sym_name: &str) -> *mut u8 {
  //TODO: switch to kernel stack
  let st = SymbolType::from(sym_type);
  match st {
    Some(st) => {
      match st {
        SymbolType::TestSymbolResolver => {
          42 as *mut u8
        }
        SymbolType::IPC => {
          //TODO: try to resolve symbol otherwise first
          match sym_name {
            "bos_set_sig_handler" => kcalls::bos_set_sig_handler as *mut u8,
            "bos_sig_handle" => kcalls::bos_sig_handle as *mut u8,
            "bos_log_trace" => kcalls::bos_log_trace as *mut u8,
            "bos_log_debug" => kcalls::bos_log_debug as *mut u8,
            "bos_log_info" => kcalls::bos_log_info as *mut u8,
            "bos_log_warn" => kcalls::bos_log_warn as *mut u8,
            "bos_log_error" => kcalls::bos_log_error as *mut u8,
            "bos_raise_page_limit" => kcalls::bos_raise_page_limit as *mut u8,
            "bos_get_page_limit" => kcalls::bos_get_page_limit as *mut u8,
            "bos_get_page_count_data" => kcalls::bos_get_page_count_data as *mut u8,
            "bos_get_page_count_nondata" => kcalls::bos_get_page_count_nondata as *mut u8,
            _ => 0 as *mut u8,
          }
        },
        _ => { panic!("symbol type not allowed yet") }
        //_ => 0 as *mut u8,
      }
    }
    None => return 0 as *mut u8,
  }
}
use ::alloc::string::String;
use ::alloc::vec::Vec;
use ::process_manager::{TaskHandle, ProcessHandle};


pub struct Process {
  name: String,
  parent: ProcessHandle,
  supervisor: ProcessHandle,
  tasks: Vec<TaskHandle>,
}

impl Process {
  pub fn new(name: String, parent: &ProcessHandle) -> Process {
    Process{
      name: name,
      parent: *parent,
      supervisor: *parent,
      tasks: Vec::new(),
    }
  }
  pub fn add_task(&mut self, t: &TaskHandle) {
    self.tasks.push(*t)
  }
}
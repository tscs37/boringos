use crate::process_manager::TaskHandle;
use crate::process_manager::state::State;
use crate::alloc::string::String;
use crate::alloc::string::ToString;

#[derive(Clone)]
pub struct Task {
  pub state: State,
  pub status: Status,
  pub parent: TaskHandle,
  pub supervisor: TaskHandle,
  pub me: TaskHandle,
  name: String,
}

impl Task {
  pub fn new<S>(s: State, parent: TaskHandle, name: S, me: TaskHandle) -> Task where S: Into<String> {
    Task {
      state: s,
      status: Status::New,
      parent,
      supervisor: parent,
      me,
      name: name.into(),
    }
  }
  #[cfg(feature = "elf_loading")]
  pub fn new_task_from_elf<S>(f: &[u8], name: S, me: TaskHandle) -> Task where S: Into<String> {
    warn!("new elf task, consider using a non-elf if possible");
    Task {
      state: State::new_elfstate(f).expect("TODO: error handle elf task generation"),
      status: Status::New,
      parent: TaskHandle::zero(),
      supervisor: TaskHandle::zero(),
      me,
      name: name.into(),
    }
  }
  pub fn new_task<S>(image: &[u8], name: S, me: TaskHandle) -> Task where S: Into<String> {
    panic!("proper tasks not implemented yet")
  }
  pub fn new_nulltask() -> Task {
    Task {
      state: State::new_nullstate(),
      status: Status::New,
      parent: TaskHandle::zero(),
      supervisor: TaskHandle::zero(),
      me: TaskHandle::zero(),
      name: "null()".to_string(),
    }
  }
  /// Copies the current task and state into a new, inactive task
  pub fn spawn(&self) -> Task {
    Task {
      state: self.state.clone(),
      status: Status::New,
      parent: self.me.clone(),
      supervisor: self.me.clone(),
      me: TaskHandle::gen(),
      name: self.name.clone(),
    }
  }
  pub fn name(&self) -> String {
    self.name.clone()
  }
  pub fn me(&self) -> TaskHandle {
    self.me
  }
  pub fn state(&self) -> &State {
    &self.state
  }
  pub fn state_mut(&mut self) -> &mut State {
    &mut self.state
  }
  pub fn rip(&self) -> u64 {
    self.state.rip()
  }
  pub fn rbp(&self) -> u64 {
    self.state.rbp()
  }
  pub fn rsp(&self) -> u64 {
    self.state.rsp()
  }
  pub fn activate(&mut self) {
    self.state.activate()
  }
  pub fn status(&self) -> Status {
    self.status
  }
  pub fn map(&self) {
    self.state.map()
  }
  pub fn state_is_null(&self) -> bool {
    self.state.mode() == crate::process_manager::state::CPUMode::Null
  }
  pub fn switch_to(&mut self, next: &mut Task) {
    trace!("state switch imminent, hold onto your hooves");
    if self.me == next.me {
      trace!("yield to self, returning");
      return;
    }
    trace!("updating task status");
    self.status = Status::Runnable;
    next.status = Status::Running;
    trace!("performing state switch");
    self.state.switch_to(next.state_mut());
    trace!("returned from state restore");
  }
}

#[derive(Copy, Clone)]
pub enum Status {
  New, // Task is new and not yet started
  Running,
  Runnable, // Task runnable
  Blocked(usize), // Blocked on Handle, TODO: Handle
  Stopped(usize), // Task stopped, 
  Shelled(usize), // Task stopped, keep alive for children
  Destroyed, // Task destroyed by kernel
  IPCFunction, // Task is a IPC Function that can be called
  Stateless, // Task is resumed at the entry point instead of the stored EIP
}
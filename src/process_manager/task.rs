use ::process_manager::TaskHandle;
use ::process_manager::state::State;
use ::alloc::string::String;
use ::alloc::prelude::ToString;

#[derive(Clone)]
pub struct Task {
  pub state: State,
  pub status: Status,
  pub parent: TaskHandle,
  pub supervisor: TaskHandle,
  name: String,
}

impl Task {
  pub fn new<S>(s: State, parent: TaskHandle, name: S) -> Task where S: Into<String> {
    Task {
      state: s,
      status: Status::New,
      parent: parent,
      supervisor: parent,
      name: name.into(),
    }
  }
  pub fn new_ktask_for_fn<S>(f: fn(), name: S) -> Task where S: Into<String> {
    warn!("new ktask, consider using a non-ktask if possible");
    debug!("ptr to fn: {:#018x}", f as u64);
    Task {
      state: State::new_kernelstate(f),
      status: Status::New,
      parent: TaskHandle::zero(),
      supervisor: TaskHandle::zero(),
      name: name.into(),
    }
  }
  pub fn new_nulltask() -> Task {
    Task {
      state: State::new_nullstate(),
      status: Status::New,
      parent: TaskHandle::zero(),
      supervisor: TaskHandle::zero(),
      name: "null()".to_string(),
    }
  }
  pub fn name(&self) -> String {
    self.name.clone()
  }
  pub fn state(&mut self) -> &mut State {
    &mut self.state
  }
  pub fn get_state_and_activate(&mut self) -> State {
    self.state.activate();
    self.state.clone()
  }
  pub fn status(&self) -> Status {
    self.status
  }
  pub fn switch_to(&mut self, next: &mut Task) {
    debug!("Switching tasks");
    self.status = Status::Runnable;
    next.status = Status::Running;
    self.state.switch_to(&mut next.state);
    panic!("returned from state restore");
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
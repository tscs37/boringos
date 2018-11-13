
use ::process_manager::state::State;

#[derive(Clone)]
pub struct Task {
  pub state: State,
  pub status: Status,
}

impl Task {
  pub fn new(s: State) -> Task {
    Task {
      state: s,
      status: Status::New,
    }
  }
  pub fn new_ktask_for_fn(f: fn()) -> Task {
    warn!("new ktask, consider using a non-ktask if possible");
    Task {
      state: State::new_kernelstate(f),
      status: Status::New,
    }
  }
  pub fn new_nulltask() -> Task {
    Task {
      state: State::new_nullstate(),
      status: Status::New,
    }
  }
  pub fn state(&mut self) -> &mut State {
    &mut self.state
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
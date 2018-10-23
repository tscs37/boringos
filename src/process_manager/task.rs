
use ::process_manager::state::State;

pub struct Task {
  state: State,
  status: Status,
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
  pub fn status(&self) -> Status {
    self.status
  }
  pub fn restore(&mut self) -> ! {
    self.status = Status::Running;
    self.state.restore();
    panic!("returned from state restore");
  }
  pub fn restore_new(&mut self) -> ! {
    self.status = Status::Running;
    self.state.restore_new();
    panic!("returned from state restore");
  }
  pub fn save_and_clear(&mut self, rsp: usize) {
    self.status = Status::Runnable;
    self.state.save_and_clear(rsp)
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
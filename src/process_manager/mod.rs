mod task;
mod process;
mod state;
mod stack;
mod handles;

#[derive(Clone)]
pub struct Userspace {
  scheduler: Arc<RefCell<Scheduler>>,
}

impl Userspace {
  pub fn new() -> Userspace {
    Userspace {
      scheduler: Arc::new(RefCell::new(Scheduler::new())),
    }
  }
  pub fn scheduler<'a>(&self) -> RefMut<'_, Scheduler> {
    (*self.scheduler).borrow_mut()
  }
  pub fn enter(&self) {
    unsafe { self.scheduler().yield_stage2(None); }
  }
  pub fn yield_to(&self, ph: Option<ProcessHandle>) {
    self.scheduler().yield_to(ph);
  }
}

use ::alloc::collections::BTreeMap;
use ::alloc::rc::Rc;
use ::alloc::sync::Arc;
use ::core::cell::{RefCell, Ref, RefMut};
pub use ::process_manager::handles::{ProcessHandle, TaskHandle, Handle};
pub use ::process_manager::process::Process;
pub use ::process_manager::task::Task;
use ::process_manager::handles::{ProcessHandleRegistry, TaskHandleRegistry};


#[derive(Clone)]
pub struct Scheduler {
  preg: Arc<RefCell<ProcessHandleRegistry>>,
  treg: Arc<RefCell<TaskHandleRegistry>>,
  scheduler_pid: ProcessHandle,
  pid_provider_pid: ProcessHandle,
}

use ::alloc::string::String;
use ::alloc::vec::Vec;

impl Scheduler {
  pub fn new() -> Scheduler {
    Scheduler {
      treg: Arc::new(RefCell::new(TaskHandleRegistry::new())),
      preg: Arc::new(RefCell::new(ProcessHandleRegistry::new())),
      scheduler_pid: ProcessHandle::from(Handle::from(0)),
      pid_provider_pid: ProcessHandle::from(Handle::from(0)),
    }
  }
  pub fn register_process(&mut self, ph: &ProcessHandle, p: Rc<RefCell<Process>>) {
    self.insert_preg(ph, p);
  }
  pub fn new_kproc(&mut self, 
    ph: &ProcessHandle, parent: &ProcessHandle, 
    name: String, f: fn()) -> Result<(), ()> {
      info!("registering kernel process '{}' ({})", name, ph);
      let mut p = Process::new(name, parent);
      let t = Task::new_ktask_for_fn(f);
      // First Task is task 0 for a KProc
      let th = &TaskHandle::from(*ph, Handle::from(0));
      p.add_task(th);
      self.insert_preg(ph, Rc::new(RefCell::new(p)));
      self.insert_treg(th, Rc::new(RefCell::new(t)));

      Ok(())
  }
  fn insert_treg(&self, th: &TaskHandle, t: Rc<RefCell<Task>>) {
    (*self.treg).borrow_mut().insert(th, t)
  }
  fn insert_preg(&self, ph: &ProcessHandle, p: Rc<RefCell<Process>>) {
    (*self.preg).borrow_mut().insert(ph, p)
  }
  // yield_to will save the current process and task context and then
  // call yield_stage2 with the given process handle
  // this function will be called by the scheduler
  pub fn yield_to(&self, ph: Option<ProcessHandle>) {
    panic!("TODO: implement yield_to");
  }
  // yield_stage2 will begin running the specified process handle
  pub unsafe fn yield_stage2(&self, ph: Option<ProcessHandle>) {
    panic!("TODO: implement yield_stage2");
  }
}
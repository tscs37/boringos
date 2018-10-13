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
  pub fn yield_to(&self, th: Option<TaskHandle>) {
    self.scheduler().yield_to(th);
  }
}

use ::alloc::rc::Rc;
use ::alloc::sync::Arc;
use ::core::cell::{RefCell, RefMut};
pub use ::process_manager::handles::{ProcessHandle, TaskHandle, Handle};
pub use ::process_manager::process::Process;
pub use ::process_manager::task::Task;
use ::process_manager::handles::{ProcessHandleRegistry, TaskHandleRegistry};


#[derive(Clone)]
pub struct Scheduler {
  preg: Arc<RefCell<ProcessHandleRegistry>>,
  treg: Arc<RefCell<TaskHandleRegistry>>,
  scheduler_thandle: TaskHandle,
  pid_provider_thandle: TaskHandle,
}

use ::alloc::string::String;

impl Scheduler {
  pub fn new() -> Scheduler {
    Scheduler {
      treg: Arc::new(RefCell::new(TaskHandleRegistry::new())),
      preg: Arc::new(RefCell::new(ProcessHandleRegistry::new())),
      scheduler_thandle: TaskHandle::from(ProcessHandle::from(Handle::from(0)), Handle::from(0)),
      pid_provider_thandle: TaskHandle::from(ProcessHandle::from(Handle::from(0)), Handle::from(0)),
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
  pub fn yield_to(&self, th: Option<TaskHandle>) {
    panic!("TODO: implement yield_to");
  }
  // yield_stage2 will begin running the specified task handle
  pub unsafe fn yield_stage2(&self, th: Option<TaskHandle>) {
    match th {
      None => self.yield_stage2(Some(self.scheduler_thandle)),
      Some(th) => {
        if let Some(task) = (*self.treg).borrow().resolve(&th) {
          use self::task::Status;
          let mut taskb = (*task).borrow_mut();
          match taskb.status() {
            Status::New => taskb.restore(),
            Status::Running => panic!("TODO: implement running non-yield"),
            Status::Runnable => panic!("TODO: implement runnable yield"),
            Status::Blocked(_) => panic!("TODO: implement blockable task"),
            Status::Stopped(_) => panic!("TODO: implement stopped task"),
            Status::Shelled(_) => self.yield_stage2(None), // Cannot be run, re-yield to scheduler
            Status::Destroyed => panic!("TODO: implement destroyed task"),
            Status::IPCFunction => self.yield_stage2(None), // Cannot be run, re-yield to scheduler
            Status::Stateless => panic!("TODO: implement stateless task"),
          }
        } else {
          self.yield_stage2(Some(
            TaskHandle::from(ProcessHandle::from(Handle::from(0)), Handle::from(0))))
        }
      }
    }
    panic!("TODO: implement yield_stage2");
  }
}
mod task;
mod process;
mod state;
mod memory;
mod handles;

use ::alloc::rc::Rc;
use ::alloc::sync::Arc;
use ::core::cell::{RefCell, RefMut, BorrowError, BorrowMutError, Ref};
pub use ::process_manager::handles::{ProcessHandle, TaskHandle, Handle};
pub use ::process_manager::process::Process;
pub use ::process_manager::task::Task;
pub use ::process_manager::state::State;
use ::process_manager::handles::{ProcessHandleRegistry, TaskHandleRegistry};
use ::process_manager::memory::Stack;

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
  pub fn scheduler<'a>(&self) -> Result<Ref<'_, Scheduler>, BorrowError> {
    (*self.scheduler).try_borrow()
  }
  pub fn scheduler_mut<'a>(&self) -> Result<RefMut<'_, Scheduler>, BorrowMutError> {
    (*self.scheduler).try_borrow_mut()
  }
  pub fn enter(&self) -> ! {
    debug!("going for yield_stage2, direct entry into PID0");
    match self.scheduler() {
      Ok(s) => unsafe { 
        debug!("mapping new kernel stack");
        (*s.kernel_stack).borrow().map();
        self.yield_to(Some(TaskHandle::from_c(0,0)));
        panic!("returned from userspace enter")
      },
      Err(p) => panic!("{}", p),
    }
  }
  pub fn yield_to(&self, th: Option<TaskHandle>) {
    match th {
      None => ::common::yield_to(0,0),
      Some(th) => ::common::yield_to(
        th.into().into() as u64, 
        th.process_handle().into().into() as u64
      ),
    }
  }
}



#[derive(Clone)]
pub struct Scheduler {
  preg: Arc<RefCell<ProcessHandleRegistry>>,
  treg: Arc<RefCell<TaskHandleRegistry>>,
  scheduler_thandle: TaskHandle,
  pid_provider_thandle: TaskHandle,
  current_task: TaskHandle, //TODO: change for multi-CPU
  kernel_stack: Arc<RefCell<Stack>>, //TODO: handle multiple kernel stacks
}

use ::alloc::string::String;


impl Scheduler {
  pub fn new() -> Scheduler {
    use alloc::prelude::ToString;
    let nullproc = Process::new("null".to_string(), &ProcessHandle::from(Handle::from(0)));
    let nulltask = Task::new_nulltask();
    let nulltaskh = TaskHandle::from(ProcessHandle::from(Handle::from(0)), Handle::from(0));
    nullproc.add_task(&nulltaskh);
    let s = Scheduler {
      treg: Arc::new(RefCell::new(TaskHandleRegistry::new())),
      preg: Arc::new(RefCell::new(ProcessHandleRegistry::new())),
      scheduler_thandle: nulltaskh,
      pid_provider_thandle: nulltaskh,
      current_task: nulltaskh,
      kernel_stack: Arc::new(RefCell::new(Stack::new_kstack())),
    };
    s.register_process(&ProcessHandle::from(Handle::from(0)), 
      Rc::new(RefCell::new(nullproc)));
    s.insert_treg(&TaskHandle::from_c(0,0), Rc::new(RefCell::new(nulltask)));
    s
  }
  pub fn current_task(&self) -> TaskHandle {
    self.current_task
  }
  pub fn register_process(&mut self, ph: &ProcessHandle, p: Rc<RefCell<Process>>) {
    self.insert_preg(ph, p);
  }
  pub fn register_scheduler(&mut self, th: &TaskHandle) {
    self.current_task = *th;
    self.scheduler_thandle = *th;
  }
  pub fn new_kproc(&mut self, 
    name: String, f: fn()) -> Result<TaskHandle, ()> {
      let ph = &ProcessHandle::gen();
      info!("registering kernel process '{}' ({})", name, ph);
      let mut p = Process::new(name, ph);
      let t = Task::new_ktask_for_fn(f);
      // First Task is task 0 for a KProc
      let h =Handle::gen();
      debug!("new task with handle {}", h);
      let th = TaskHandle::from(*ph, h);
      p.add_task(&th);
      self.insert_preg(ph, Rc::new(RefCell::new(p)));
      self.insert_treg(&th, Rc::new(RefCell::new(t)));

      Ok(th)
  }
  fn insert_treg(&self, th: &TaskHandle, t: Rc<RefCell<Task>>) {
    (*self.treg).borrow_mut().insert(th, t)
  }
  fn insert_preg(&self, ph: &ProcessHandle, p: Rc<RefCell<Process>>) {
    (*self.preg).borrow_mut().insert(ph, p)
  }
  pub fn resolve_th(&self, th: &TaskHandle) -> Option<Rc<RefCell<Task>>> {
    (*self.treg).borrow().resolve(th).and_then(|x| Some(x.clone()))
  }
  pub fn resolve_ph(&self, ph: &ProcessHandle) -> Option<Rc<RefCell<Process>>> {
    (*self.preg).borrow().resolve(ph).and_then(|x| Some(x.clone()))
  }
  // yield_to will save the current process and task context and then
  // call yield_stage2 with the given process handle
  // this function will be called by the scheduler
  pub fn yield_to(&self, rsp: usize, th: Option<TaskHandle>) {
    //pivot_to_kernel_stack!();
    debug!("entering scheduler code");
    match th {
      None => {
        let sched = self.scheduler_thandle;
        self.yield_stage2_sched_internal(sched);
      }
      Some(th) => { 
        debug!("updating current task to {}", th);
        let us = ::userspace();
        {
          match us.scheduler_mut() {
            Err(_) => panic!("need scheduler mutably for updating current task"),
            Ok(ref mut s) => { s.current_task = th },
          }
        }
        self.yield_stage2_sched_internal(th)
      },
    }
  }
  pub unsafe fn yield_stage2_sched(self, th: TaskHandle) {
      self.yield_stage2_sched_internal(th);
  }
  unsafe fn yield_stage2_sched_internal(&self, th: TaskHandle) {
    debug!("entering scheduler stage 2 final for {}", th);
    if let Some(task) = (*self.treg).borrow().resolve(&th) {
      use self::task::Status;
      let mut taskb = (*task).borrow_mut();
      if let Some(taskc) = (*self.treg).borrow().resolve(&self.current_task) {
        self.current_task = th;
        match taskb.status() {
          Status::New => (*taskc).borrow_mut().switch_to(taskb),
          _ => panic!("TODO: implement returning from new tasks"),
        };
      }
      panic!("current task did not resolve")
    } else {
      if th.into().into() == 0 && th.process_handle().into().into() == 0 {
        let sched = self.scheduler_thandle;
        if sched.into().into() == 0 && th.process_handle().into().into() == 0 {
          panic!("attempted to yield but no process manager is present");
        }
        self.yield_stage2_sched_internal(sched);
      } else {
        panic!("tried to yield to non-existant task handle {}", th);
      }
    }
  }
}
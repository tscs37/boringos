mod task;
mod process;
mod state;
mod stack;
mod handles;

use ::alloc::rc::Rc;
use ::alloc::sync::Arc;
use ::core::cell::{RefCell, RefMut, BorrowError, BorrowMutError, Ref};
pub use ::process_manager::handles::{ProcessHandle, TaskHandle, Handle};
pub use ::process_manager::process::Process;
pub use ::process_manager::task::Task;
use ::process_manager::handles::{ProcessHandleRegistry, TaskHandleRegistry};
use ::process_manager::stack::Stack;

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
        s.yield_stage2(None)
      },
      Err(p) => panic!("{}", p),
    }
  }
  pub fn yield_to(&self, th: Option<TaskHandle>) {
    match th {
      None => ::common::yield_to(0,0),
      Some(th) => ::common::yield_to(
        th.into().into() as usize, 
        th.process_handle().into().into() as usize
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
    let null = TaskHandle::from(ProcessHandle::from(Handle::from(0)), Handle::from(0));
    Scheduler {
      treg: Arc::new(RefCell::new(TaskHandleRegistry::new())),
      preg: Arc::new(RefCell::new(ProcessHandleRegistry::new())),
      scheduler_thandle: null,
      pid_provider_thandle: null,
      current_task: null,
      kernel_stack: Arc::new(RefCell::new(Stack::new_kstack())),
    }
  }
  pub fn current_task(&self) -> TaskHandle {
    self.current_task
  }
  pub fn register_process(&mut self, ph: &ProcessHandle, p: Rc<RefCell<Process>>) {
    self.insert_preg(ph, p);
  }
  pub fn register_scheduler(&mut self, th: &TaskHandle) {
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
  // yield_stage2 will begin running the specified task handle
  pub unsafe fn yield_stage2(&self, th: Option<TaskHandle>) -> ! {
    debug!("entering scheduler stage 2");
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
  pub unsafe fn yield_stage2_sched(self, th: TaskHandle) -> ! {
      self.yield_stage2_sched_internal(th);
  }
  unsafe fn yield_stage2_sched_internal(&self, th: TaskHandle) -> ! {
    debug!("entering scheduler stage 2 final for {}", th);
    if let Some(task) = (*self.treg).borrow().resolve(&th) {
      use self::task::Status;
      let mut taskb = (*task).borrow_mut();
      match taskb.status() {
        Status::New => taskb.restore_new(),
        _ => panic!("TODO: implement returning from new tasks"),
      };
    } else {
      if th.into().into() == 0 && th.process_handle().into().into() == 0 {
        let sched = self.scheduler_thandle;
        self.yield_stage2_sched_internal(sched);
      }
      panic!("tried to yield to non-existant task handle {}", th);
    }
  }
}
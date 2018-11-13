mod task;
mod process;
mod state;
mod memory;
mod handles;

use ::alloc::sync::Arc;
pub use ::process_manager::handles::{ProcessHandle, TaskHandle, Handle};
pub use ::process_manager::process::Process;
pub use ::process_manager::task::Task;
pub use ::process_manager::state::State;
use ::process_manager::handles::{ProcessHandleRegistry, TaskHandleRegistry};
use ::process_manager::memory::Stack;
use ::spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct Userspace {
  scheduler: Arc<RwLock<Scheduler>>,
}

impl Userspace {
  pub fn new() -> Userspace {
    Userspace {
      scheduler: Arc::new(RwLock::new(Scheduler::new())),
    }
  }
  pub fn in_scheduler<T>(&self, run: impl Fn(RwLockReadGuard<Scheduler>)->T) -> Result<T, ()> {
    match (*self.scheduler).try_read() {
      None => Err(()),
      Some(sched) => Ok(run(sched)),
    }
  }
  pub fn in_scheduler_spin<T>(&self, run: impl Fn(RwLockReadGuard<Scheduler>)->T) -> T {
    run((*self.scheduler).read())
  }
  pub fn in_scheduler_mut<T>(&self, run: impl Fn(RwLockWriteGuard<Scheduler>)->T) -> Result<T, ()> {
    match (*self.scheduler).try_write() {
      None => Err(()),
      Some(sched) => Ok(run(sched)),
    }
  }
  pub fn in_scheduler_mut_spin<T>(&self, run: impl Fn(RwLockWriteGuard<Scheduler>)->T) -> T {
    run((*self.scheduler).write())
  }
  pub fn enter(&self) -> ! {
    debug!("entry into PID0");
    let sched = self.in_scheduler_spin(|sched| {
      (*sched.kernel_stack).read().map();
      let sched_th = (*sched).scheduler_thandle;
      let sched_task = (*sched).resolve_th(&sched_th);
      *(sched_task.expect("entering userspace requires scheduler"))
    });
    unsafe { ::process_manager::state::switch_to(&mut *sched) };
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
  preg: Arc<RwLock<ProcessHandleRegistry>>,
  treg: Arc<RwLock<TaskHandleRegistry>>,
  scheduler_thandle: TaskHandle,
  current_task: TaskHandle, //TODO: change for multi-CPU
  kernel_stack: Arc<RwLock<Stack>>, //TODO: handle multiple kernel stacks
}

use ::alloc::string::String;


impl Scheduler {
  pub fn new() -> Scheduler {
    use alloc::prelude::ToString;
    let mut nullproc = Process::new("null".to_string(), &ProcessHandle::from(Handle::from(0)));
    let mut nulltask = Task::new_nulltask();
    let nulltaskh = TaskHandle::from(ProcessHandle::from(Handle::from(0)), Handle::from(0));
    nullproc.add_task(&nulltaskh);
    let mut s = Scheduler {
      treg: Arc::new(RwLock::new(TaskHandleRegistry::new())),
      preg: Arc::new(RwLock::new(ProcessHandleRegistry::new())),
      scheduler_thandle: nulltaskh,
      current_task: nulltaskh,
      kernel_stack: Arc::new(RwLock::new(Stack::new_kstack())),
    };
    s.register_process(&ProcessHandle::from(Handle::from(0)), 
      nullproc);
    s.insert_treg(&TaskHandle::from_c(0,0), &mut nulltask);
    s
  }
  pub fn current_task(&self) -> TaskHandle {
    self.current_task
  }
  pub fn register_process(&mut self, ph: &ProcessHandle, p: Process) {
    self.insert_preg(ph, p);
  }
  pub fn register_scheduler(&mut self, th: &TaskHandle) {
    //self.current_task = *th;
    self.scheduler_thandle = *th;
  }
  pub fn new_kproc(&mut self, 
    name: String, f: fn()) -> Result<TaskHandle, ()> {
      let ph = &ProcessHandle::gen();
      info!("registering kernel process '{}' ({})", name, ph);
      let mut p = Process::new(name, ph);
      let mut t = Task::new_ktask_for_fn(f);
      // First Task is task 0 for a KProc
      let h =Handle::gen();
      debug!("new task with handle {}", h);
      let th = TaskHandle::from(*ph, h);
      p.add_task(&th);
      self.insert_preg(ph, p);
      self.insert_treg(&th, &mut t);

      Ok(th)
  }
  fn insert_treg(&self, th: &TaskHandle, t: &mut Task) {
    (*self.treg).write().insert(th, t)
  }
  fn insert_preg(&self, ph: &ProcessHandle, p: Process) {
    (*self.preg).write().insert(ph, p)
  }
  pub fn resolve_th(&self, th: &TaskHandle) -> Option<Arc<*mut Task>> {
    (*self.treg).read().resolve(th).and_then(|x| Some(x.clone()))
  }
  pub fn resolve_ph(&self, ph: &ProcessHandle) -> Option<Arc<RwLock<Process>>> {
    (*self.preg).read().resolve(ph).and_then(|x| Some(x.clone()))
  }
  // yield_to will save the current process and task context and then
  // call yield_stage2 with the given process handle
  // this function will be called by the scheduler
  pub fn yield_to(&mut self, th: Option<TaskHandle>) {
    dump_stack_addr!();
    match th {
      None => {
        let sched = self.scheduler_thandle;
        self.yield_to(Some(sched));
        //unsafe { self.yield_stage2_sched_internal(sched) };
      }
      Some(th_in) => { 
        let th;
        if th_in.is_scheduler() {
          let sched = self.scheduler_thandle.clone();
          debug!("task handle is scheduler, swapping for {}", sched);
          th = sched;
        } else {
          th = th_in;
        }
        assert_ne!(self.current_task, th, "cannot yield to yourself");
        let current_task: Arc<*mut Task>;
        let next_task: Arc<*mut Task>;
        {
          let treg = (*self.treg).read();
          current_task = (treg.resolve(&self.current_task).unwrap()).clone();
          next_task = (treg.resolve(&th).unwrap()).clone();
        }
        self.current_task = th;
        use self::task::Status;
        let status;
        {
          let nt = unsafe{&**next_task};
          status = nt.status();
          drop(nt);
        }
        debug!("Got next and current task, switching context");
        match status {
          Status::New => {
            { 
              debug!("Doing switch...");
              unsafe {
                let mut ct = &mut (**current_task);
                let mut nt = &mut (**next_task);
                ct.switch_to(nt);
              }
            }
          },
          _ => panic!("TODO: implement resuming tasks"),
        }
      },
    }
  }
}
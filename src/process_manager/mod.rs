mod handles;
mod memory;
mod state;
mod task;

use alloc::sync::Arc;
use core::cell::RefCell;
use crate::process_manager::handles::TaskHandleRegistry;
pub use crate::process_manager::handles::{Handle, TaskHandle};
pub use crate::process_manager::memory::{
  Memory, MemoryKernel, MemoryKernelRef, MemoryUser, MemoryUserRef,
};
pub use crate::process_manager::state::State;
pub use crate::process_manager::task::Task;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct Userspace {
  scheduler: Arc<RwLock<Scheduler>>,
}

panic_on_drop!(Userspace);

impl Userspace {
  pub fn new() -> Userspace {
    Userspace {
      scheduler: Arc::new(RwLock::new(Scheduler::new())),
    }
  }
  pub fn in_scheduler<T>(&self, run: impl Fn(RwLockReadGuard<Scheduler>) -> T) -> Result<T, ()> {
    match (*self.scheduler).try_read() {
      None => Err(()),
      Some(sched) => Ok(run(sched)),
    }
  }
  pub fn in_scheduler_spin<T>(&self, run: impl Fn(RwLockReadGuard<Scheduler>) -> T) -> T {
    run((*self.scheduler).read())
  }
  pub fn in_scheduler_mut<T>(
    &self,
    mut run: impl FnMut(RwLockWriteGuard<Scheduler>) -> T,
  ) -> Result<T, ()> {
    match (*self.scheduler).try_write() {
      None => Err(()),
      Some(sched) => Ok(run(sched)),
    }
  }
  pub fn in_scheduler_mut_spin<T>(&self, mut run: impl FnMut(RwLockWriteGuard<Scheduler>) -> T) -> T {
    run((*self.scheduler).write())
  }
  pub fn enter(&self) {
    debug!("entry into PID0");
    let sched = self
      .in_scheduler(|sched| {
        (*sched.kernel_stack).read().map();
        let sched_th = (sched).scheduler_thandle;
        let sched_task = (sched).resolve_th(sched_th);
        ( sched_task.expect("entering userspace requires scheduler"), sched_th )
      })
      .expect("userspace enter needs lock acquire");
    self.in_scheduler_mut(|mut scheduler| {
      scheduler.current_task = sched.1;
    }).expect("require scheduler to enter userspace");
    unsafe { crate::process_manager::state::switch_to(sched.0, sched.1) }
  }
  pub fn yield_to(&self, th: Option<TaskHandle>) {
    use crate::common::yield_to;
    match th {
      None => yield_to(0),
      Some(th) => yield_to(th.into_c()),
    }
  }
}

use alloc::string::String;

#[derive(Clone)]
pub struct Scheduler {
  treg: Arc<RwLock<TaskHandleRegistry>>,
  scheduler_thandle: TaskHandle,
  current_task: TaskHandle,         //TODO: change for multi-CPU
  kernel_stack: Arc<RwLock<Memory>>, //TODO: handle multiple kernel stacks
}

panic_on_drop!(Scheduler);

impl Scheduler {
  pub fn new() -> Scheduler {
    let nulltask = Task::new_nulltask();
    let s = Scheduler {
      treg: Arc::new(RwLock::new(TaskHandleRegistry::new())),
      scheduler_thandle: nulltask.me,
      current_task: nulltask.me,
      kernel_stack: Arc::new(RwLock::new(Memory::new_kernelstack())),
    };
    s.insert_treg(nulltask);
    s
  }
  pub fn current_task(&self) -> TaskHandle {
    self.current_task
  }
  pub fn set_current_task(&mut self, th: TaskHandle) {
    self.current_task = th
  }
  pub fn spawn_from_task(&mut self) -> Option<TaskHandle> {
    let task = self.resolve_th(self.current_task())?;
    let new_task = (*task).borrow().spawn();
    let new_task_th = new_task.me;
    info!("new task spawned from {} to {}", (*task).borrow().me, new_task_th);
    Some(self.insert_treg(new_task))
  }
  pub fn register_scheduler(&mut self, th: TaskHandle) {
    self.scheduler_thandle = th;
  }
  pub fn new_elfproc<S>(&mut self, name: S, f: &[u8]) -> Result<TaskHandle, ()>
  where
    S: Into<String>,
  {
    let n = name.into();
    let th = TaskHandle::gen();
    debug!("new task with handle {}", th);
    let t = Task::new_task_from_elf(f, n.clone(), th);
    self.insert_treg(t);
    info!("registered kernel process '{}' ({})", n, th);
    Ok(th)
  }
  fn insert_treg(&self, t: Task) -> TaskHandle{
    let me = t.me;
    (*self.treg).write().insert(me, t);
    me
  }
  pub fn resolve_th(&self, th: TaskHandle) -> Option<Arc<RefCell<Task>>> {
    (*self.treg)
      .read()
      .resolve(th)
      .and_then(|x| Some((*x).clone()))
  }
  // yield_to will save the current process and task context and then
  // call yield_stage2 with the given process handle
  // this function will be called by the scheduler
  pub fn yield_to(&self, cur: TaskHandle, th: Option<TaskHandle>) {
    dump_stack_addr!();
    match th {
      None => {
        let sched = self.scheduler_thandle;
        self.yield_to(cur, Some(sched));
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
        assert_eq!(th, self.current_task, "caller must update current task");
        if th == cur {
          // if the task is already running, do nothing and return
          return;
        }
        let current_task: Arc<RefCell<Task>>;
        let next_task: Arc<RefCell<Task>>;
        {
          let treg = (*self.treg).read();
          current_task = (treg.resolve(cur).expect("need current task")).clone();
          next_task = (treg.resolve(th).expect("need next task")).clone();
        }
        use self::task::Status;
        let status;
        {
          let nt = next_task.borrow();
          status = nt.status();
          drop(nt);
        }
        debug!("Got next and current task, switching context");
        match status {
          Status::New => {
            debug!("Doing switch...");
            trace!("grabbing current task");
            let mut ct = current_task.borrow_mut();
            trace!("grabbing next task");
            let mut nt = next_task.borrow_mut();
            trace!("executing switch");
            ct.switch_to(&mut nt);
          }
          _ => panic!("TODO: implement resuming tasks"),
        }
      }
    }
  }
}

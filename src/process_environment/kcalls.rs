use crate::*;

pub fn bos_set_sig_handler(f: *mut u8) {
  debug!("set signal handler here: {:?}", f)
}

pub fn bos_sig_handle(sig: u64, id: u64, code: u64) {
  debug!("handle signal handling here")
}

pub fn bos_log_trace(msg: &str) {
  trace!("{}", msg);
}

pub fn bos_log_trace_fmt(msg: core::fmt::Arguments) {
  debug!("{}", msg);
}

pub fn bos_log_debug(msg: &str) {
  debug!("{}", msg);
}

pub fn bos_log_debug_fmt(msg: core::fmt::Arguments) {
  debug!("{}", msg);
}

pub fn bos_log_info(msg: &str) {
  info!("{}", msg);
}

pub fn bos_log_info_fmt(msg: core::fmt::Arguments) {
  info!("{}", msg);
}

pub fn bos_log_warn(msg: &str) {
  warn!("{}", msg);
}

pub fn bos_log_warn_fmt(msg: core::fmt::Arguments) {
  warn!("{}", msg);
}

pub fn bos_log_error(msg: &str) {
  error!("{}", msg);
}

pub fn bos_log_error_fmt(msg: core::fmt::Arguments) {
  error!("{}", msg);
}

pub fn bos_yield(th: u64) {
  panic!("TODO: implement yield")
}

// bos_raise_page_limit raises the amount of memory the program may use
// This limit includes code, stack, bss and data memory by default.
// Each call may raise the limit by up to 256MB.
// This does not mean the OS is able to allocate these pages
// The call returns the new page limit
pub fn bos_raise_page_limit(pages: u16) -> u64 {
  trace!("raising page limit by {}", pages);
  with_current_task_mut(|task| {
    match task {
      None => 0,
      Some(mut task) => task.state_mut().raise_page_limit(pages),
    }
  }).unwrap_or_default()
}

// Returns the current page limit
pub fn bos_get_page_limit() -> u64 {
  with_current_task(|task| {
    match task {
      None => 0,
      Some(task) => task.state().page_limit(),
    }
  }).unwrap_or_default()
}

// Returns number of pages allocated for data, not counting stack or code
pub fn bos_get_page_count_data() -> u64 {
  kinfo().get_data_memory_ref_size() as u64
}

// Returns number of pages allocated for non-data, counting stack and code
pub fn bos_get_page_count_nondata() -> u64 {
  kinfo().get_code_memory_ref_size() as u64
  + kinfo().get_stack_memory_ref_size() as u64
}

// bos_promise_pages will allocate a number of pages to the program beyond
// the currently allocated ones. The returned number is how many pages
// the OS is able to actually promise.
// The call will fail and return 0 if the page limit is exceeded.
// After this call, the program will be able to use as many pages as the OS
// returned, until the task terminates.
pub fn bos_promise_pages(pages: u16) -> u16 {
  panic!("TODO:")
}

// creates a new task and returns the handle for the task
// The memory section referred to by code_ is loaded into non-writable, executable memory
// The memory section referred to by bss_ is loaded into writable, non-executable memory
// bss is not the same as data memory, the bss memory cannot be expanded after the task is created
// The task is created in a paused New state, which means the scheduler will not implicitly execute it
// To run the task, simply explicitly yield to it.
pub fn bos_new_task(code_ptr: *mut u8, code_size: u32, bss_ptr: *mut u8, bss_size: u32) -> u64 {
  panic!("TODO:")
}

// Unlike bos_new_task this will create a runnable task.
// Return behaviour differs by where it returns. The newly spawned task
// will receive the task handle of the previously existing task and the previously
// existing task will receive the task handle of the newly spawned task.
pub fn bos_copy_task() -> u64 {
  panic!("TODO:")
}

// kills and destroys the given task handle
// the task will be notified via a SIGTERM signal event
// the receiving task will be terminated when the signal handler
// returns or the signal handler times out.
pub fn bos_destroy_task(th: u64) {
  panic!("TODO:")
}

// returns the current task handle
pub fn bos_own_th() -> u64 {
  panic!("TODO:")
}

// Create a codeimage; this is a section of memory that will be copied into a dedicated
// pages of memory and can be used exactly once. If the code image is assigned to a task
// the handle is invalidated
pub fn bos_make_codeimage(ptr: *mut u8, size: u32) -> u64 {
  panic!("TODO:")
}

pub fn bos_set_scheduler(th: u64) {
  panic!("TODO:")
}

pub fn bos_add_event_handler(intr: u16, ptr: *mut u8) {
  panic!("TODO:")
}

pub fn bos_register_ipc(ptr: *mut u8) -> u64 {
  panic!("TODO:")
}
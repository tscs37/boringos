//! System calls.

/// Voluntarily give a time slice to the scheduler.
#[cfg(target_os = "boringos")]
pub fn sched_yield() -> usize {
  import_symbol!(bos_log_trace, fn(&str));
  bos_log_trace("yielding to processor");
  import_symbol!(bos_yield, fn(u64));
  bos_yield(0);
  0
}

/// Change the data segment. See `man brk`.
///
/// On success, the new program break is returned. On failure, the old program break is returned.
///
/// # Note
///
/// This is the `brk` **syscall**, not the library function.
#[cfg(target_os = "boringos")]
pub unsafe fn brk(ptr: *const u8) -> *const u8 {
  const OFFSET: u64 = 0x0000_01f0_0000_0000;
  import_symbol!(bos_get_page_count_data, fn() -> u64);
  import_symbol!(bos_raise_page_limit, fn(u16) -> u64);
  if ptr as u64 == 0 {
    return (bos_get_page_count_data() * 4096 + OFFSET) as *const u8;
  }
  let tar_datasize = ((ptr as u64 - OFFSET) / 4096) + 1;
  let cur_datasize = bos_get_page_count_data();
  let new_pages = if (tar_datasize > cur_datasize) {
    let inc_pages = tar_datasize - cur_datasize;
    // bos_raise_page_limit returns the total pagelimit, not the number of datapages
    bos_raise_page_limit(inc_pages as u16);
    let new_datasize = bos_get_page_count_data();
    new_datasize - cur_datasize
  } else {
    //TODO: free memory if ptr significantly (>2 pages) below current limit
    0
  };
  let ptr = ptr as u64 + (new_pages * 4096);
  ptr as *const u8
}
//! System calls.

/// Voluntarily give a time slice to the scheduler.
#[cfg(target_os = "boringos")]
pub fn sched_yield() -> usize {
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
  import_symbol!(bos_get_page_count_data, fn() -> u64);
  import_symbol!(bos_raise_page_limit, fn(u16) -> u64);
  let tar_datasize = (ptr as u64 / 4096) + 1;
  let cur_datasize = bos_get_page_count_data();
  let inc_pages = tar_datasize - cur_datasize;
  let new_datasize = if inc_pages > core::u16::MAX as u64 {
    let mut inc = 0;
    for _ in 0..inc_pages {
      inc += bos_raise_page_limit(inc_pages.saturating_sub(inc) as u16);
    }
    inc
  } else {
    bos_raise_page_limit(inc_pages as u16)
  };
  let new_pages = new_datasize - cur_datasize;
  (ptr as u64 + (new_pages * 4096)) as *const u8
}

use x86_64::structures::paging::{PageTable, OffsetPageTable};
use x86_64::VirtAddr;

use crate::*;

unsafe fn active_level4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
  use x86_64::registers::control::Cr3;

  let (level_4_table_frame, _) = Cr3::read();

  let phys = level_4_table_frame.start_address();
  let virt = VirtAddr::new(phys.as_u64() + physical_memory_offset.as_u64());
  let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

  &mut *page_table_ptr
}

pub type Mapper = OffsetPageTable<'static>;

pub unsafe fn init<'a>(physical_memory_offset: VirtAddr) {
  trace!("initializing page map in kinfo");
  let level_4_table = active_level4_table(physical_memory_offset);
  kinfo_mut().set_page_table(
    OffsetPageTable::new(level_4_table, physical_memory_offset)
  );
}

pub fn get_pagemap<T, F>(run: F) -> Option<T> where F: for<'a> Fn(&'a Mapper) -> T {
  kinfo().get_page_table(run)
}

pub fn get_pagemap_mut<T, F>(run: F) -> Option<T> where F: for<'a> Fn(&'a mut Mapper) -> T {
  kinfo().get_page_table_mut(run)
}


pub use x86_64::structures::paging::{Page, PageTableFlags};
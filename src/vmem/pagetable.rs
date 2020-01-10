
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
  kinfo_mut().set_pmo(physical_memory_offset);
}

use spin::RwLock;

lazy_static!{
  static ref LOCK: RwLock<()> = RwLock::default();
}

pub fn get_pagemap<T, F>(run: F) -> T where F: for<'a> Fn(&'a Mapper) -> T {
  let lock = LOCK.read();
  let physical_memory_offset = kinfo().get_pmo();
  let level_4_table = unsafe{active_level4_table(physical_memory_offset)};
  let opt = unsafe{OffsetPageTable::new(level_4_table, physical_memory_offset)};
  let ret = run(&opt);
  drop(lock);
  ret
}

pub fn get_pagetable<F>(run: F) where F: for<'a> Fn(&'a PageTable) {
  let lock = LOCK.read();
  let physical_memory_offset = kinfo().get_pmo();
  let level_4_table = unsafe{active_level4_table(physical_memory_offset)};
  run(&level_4_table);
  drop(lock);
}

pub fn get_pagemap_mut<T, F>(mut run: F) -> T where F: for<'a> FnMut(&'a mut Mapper) -> T {
  let lock = LOCK.read();
  let physical_memory_offset = kinfo().get_pmo();
  let level_4_table = unsafe{active_level4_table(physical_memory_offset)};
  let mut opt = unsafe{OffsetPageTable::new(level_4_table, physical_memory_offset)};
  let ret = run(&mut opt);
  drop(lock);
  ret
}


pub use x86_64::structures::paging::{Page, PageTableFlags};
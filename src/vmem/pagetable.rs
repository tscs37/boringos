
use x86_64::structures::paging::{PageTable, PhysFrame, MappedPageTable};
use x86_64::VirtAddr;

use crate::*;

unsafe fn active_level4_table(physical_memory_offset: u64) -> &'static mut PageTable {
  use x86_64::registers::control::Cr3;

  let (levell_4_table_frame, _) = Cr3::read();

  let phys = levell_4_table_frame.start_address();
  let virt = VirtAddr::new(phys.as_u64() + physical_memory_offset);
  let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

  &mut *page_table_ptr
}

pub type Mapper<'a> = MappedPageTable<'a, &'a(fn(PhysFrame) -> *mut PageTable)> ;

pub unsafe fn init<'a>(physical_memory_offset: u64) -> Mapper<'a>  {
  trace!("initializing page map in kinfo");
  kinfo_mut().set_vmem_boot_offset(physical_memory_offset);
  trace!("returning page map from kinfo");
  get_pagemap()
}

fn translate_phys_to_virt(frame: PhysFrame) -> *mut PageTable {
  let phys = frame.start_address().as_u64();
  let virt = VirtAddr::new(phys + kinfo().get_vmem_boot_offset());
  virt.as_mut_ptr()
}

pub fn get_pagemap<'a>() -> Mapper<'a> {
  trace!("reading pagetable");
  let level_4_table = unsafe{active_level4_table(kinfo().get_vmem_boot_offset())};
  unsafe{MappedPageTable::new(level_4_table, &(translate_phys_to_virt as fn(PhysFrame) -> *mut PageTable))}
}


pub use x86_64::structures::paging::{Page, PageTableFlags};
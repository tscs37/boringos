use alloc::vec::Vec;
use crate::process_manager::TaskHandle;
use crate::vmem::pagetable::Page;
use crate::vmem::PhysAddr;
use crate::vmem::PAGE_SIZE;
use crate::*;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::mapper::MapperAllSizes;
use x86_64::structures::paging::mapper::Mapper;
use vmem::pagetable::{get_pagemap, get_pagemap_mut};

#[derive(PartialEq, Debug)]
pub enum MapType {
  Stack,               // Stack Page, No Execute
  Data,                // Data Page, No Execute
  Code,                // Code Page, No Write
  ReadOnly,            // Data Page, No Write
  Managed(TaskHandle), // Memory available via other process
  ShMem(TaskHandle),   // Memory shared to other process
  Guard,               // No Execute, No Read+Write
  Zero,                // Map No Execute, No RW Page, share page
}

impl MapType {
  pub fn flags(&self) -> PageTableFlags {
    let flags = match self {
      MapType::Stack => PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
      MapType::Data => PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE,
      MapType::Code => PageTableFlags::empty(),
      MapType::ReadOnly => PageTableFlags::NO_EXECUTE,
      MapType::Managed(_) => PageTableFlags::NO_EXECUTE,
      MapType::ShMem(_) => PageTableFlags::NO_EXECUTE,
      MapType::Guard => PageTableFlags::NO_EXECUTE,
      MapType::Zero => PageTableFlags::NO_EXECUTE,
    };
    flags | PageTableFlags::PRESENT
  }
}

use x86_64::structures::paging::PhysFrame;

pub fn map_new(base_addr: VirtAddr, mt: MapType) -> PhysAddr {
  trace!("mapping new page to {:?} ({:?})", base_addr, mt);
  let pm = &mut pager();
  let flags = mt.flags();
  let frame = unsafe{ pm.alloc_page().expect("map new failed") };
  let page: Page<Size4KiB> = Page::containing_address(base_addr);
  let pagepool = &mut pm.pagepool_raw_mut().clone();
  get_pagemap_mut(|apt| {
    let res = unsafe { apt.map_to(page, PhysFrame::containing_address(frame), 
      flags, pagepool) };
    let res = res.unwrap();
    res.flush();
    frame
  }).expect("map_new failed")
}

pub fn map_zero(addr: VirtAddr, size: u32) {
  // grab zero_page first, otherwise we get a problem when we grab the lock on the APT
  // below!
  trace!("mapping memory at {:?} ({} pages, {:?})", addr, size, MapType::Zero);
  let zero_page = kinfo().get_zero_page_addr();
  let pm = &mut pager();
  let pagepool = &mut pm.pagepool_raw_mut().clone();
  get_pagemap_mut(|apt| {
    let flags = MapType::Zero.flags();
    let page: Page<Size4KiB> = Page::containing_address(addr);
    for x in 0..size {
      let addr = addr + x as usize * PAGE_SIZE;
      trace!("map zero: {:?}", addr);
      unsafe { apt.map_to(
        page,
        PhysFrame::containing_address(zero_page),
        flags,
        pagepool,
      )} .unwrap().flush()
    }
  }).expect("map zero failed")
}

pub fn is_mapped(addr: VirtAddr) -> bool {
  trace!("checking if {:?} is mapped", addr);
  get_pagemap(|apt| {
    apt.translate_addr(addr).is_some()
  }).expect("is_mapped failed")
}

pub fn map(base_addr: VirtAddr, pl: Vec<PhysAddr>, mt: MapType) {
  trace!("mapping memory at {:?} ({} pages, {:?})", base_addr, pl.len(), mt);
  let pm = &mut pager();
  let pagepool = &mut pm.pagepool_raw_mut().clone();
  get_pagemap_mut(|apt| {
    let flags = mt.flags();
    for x in 0..pl.len() {
      let addr: VirtAddr = if mt == MapType::Stack {
        base_addr - x * PAGE_SIZE
      } else {
        base_addr + x * PAGE_SIZE
      };
      trace!("map: {:?}", addr);
      let page: Page<Size4KiB> = Page::containing_address(addr);
      unsafe { apt.map_to(page, PhysFrame::containing_address(pl[x]), flags, pagepool) }
        .expect("must not fail map").flush();
    }
  }).expect("map failed")
}

pub fn unmap(base_addr: VirtAddr, pl_size: usize, mt: MapType) {
  trace!("unmapping memory at {:?} ({} pages, {:?})", base_addr, pl_size, mt);
  get_pagemap_mut(|apt| {
    for x in 0..pl_size {
      let addr: VirtAddr = if mt == MapType::Stack {
        base_addr - x * PAGE_SIZE
      } else {
        base_addr + x * PAGE_SIZE
      };
      trace!("unmap: {:?}", addr);
      let page: Page<Size4KiB> = Page::containing_address(addr);
      apt.unmap(page).expect("unmap failed").1.flush();
    }
  }).expect("unmap failed")
}

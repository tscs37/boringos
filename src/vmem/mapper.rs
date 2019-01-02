use alloc::vec::Vec;
use crate::process_manager::TaskHandle;
use crate::vmem::pagetable::Page;
use crate::vmem::pagetable::{ActivePageTable, EntryFlags};
use crate::vmem::PhysAddr;
use crate::vmem::PAGE_SIZE;

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
  fn flags(&self) -> EntryFlags {
    match self {
      MapType::Stack => EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE,
      MapType::Data => EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE,
      MapType::Code => EntryFlags::NOTHING,
      MapType::ReadOnly => EntryFlags::NO_EXECUTE,
      MapType::Managed(_) => EntryFlags::OS_EXTERNAL,
      MapType::ShMem(_) => EntryFlags::OS_EXTERNAL,
      MapType::Guard => EntryFlags::NO_EXECUTE,
      MapType::Zero => EntryFlags::NO_EXECUTE,
    }
  }
}

pub fn map_new(base_addr: PhysAddr, mt: MapType) -> PhysAddr {
  let mut apt = unsafe { ActivePageTable::new() };
  let pm = &mut crate::pager();
  let flags = mt.flags();
  trace!("mapping new page to {} ({:?})", base_addr, mt);
  apt.map(Page::containing_address(base_addr.as_usize()), flags, pm)
}

pub fn map_zero(addr: PhysAddr, size: u16) {
  // grab zero_page first, otherwise we get a problem when we grab the lock on the APT
  // below!
  trace!("mapping memory at {} ({} pages, {:?})", addr, size, MapType::Zero);
  let zero_page = crate::kinfo().get_zero_page_addr();
  let mut apt = unsafe { ActivePageTable::new() };
  let pm = &mut crate::pager();
  let flags = MapType::Zero.flags();
  for x in 0..size {
    let addr = addr.as_usize() + x as usize * crate::vmem::PAGE_SIZE;
    trace!("map zero: {:#018x}", addr);
    apt.map_to(
    Page::containing_address(addr),
    zero_page,
    flags,
    pm,
  )
  }
}

pub fn is_mapped(addr: PhysAddr) -> bool {
  let apt = unsafe { ActivePageTable::new() };
  apt.translate(addr.as_usize()).is_some()
}

pub fn map(base_addr: PhysAddr, pl: Vec<PhysAddr>, mt: MapType) {
  trace!("mapping memory at {} ({} pages, {:?})", base_addr, pl.len(), mt);
  let mut apt = unsafe { ActivePageTable::new() };
  let pm = &mut crate::pager();
  let flags = mt.flags();
  for x in 0..pl.len() {
    let addr = if mt == MapType::Stack {
      base_addr.as_usize() - x * PAGE_SIZE
    } else {
      base_addr.as_usize() + x * PAGE_SIZE
    };
    trace!("map: {:#018x}", addr);
    apt.map_to(Page::containing_address(addr), pl[x], flags, pm);
  }
}

pub fn unmap(base_addr: PhysAddr, pl_size: usize, mt: MapType) {
  trace!("unmapping memory at {} ({} pages, {:?})", base_addr, pl_size, mt);
  let mut apt = unsafe { ActivePageTable::new() };
  let pm = &mut crate::pager();
  for x in 0..pl_size {
    let addr = if mt == MapType::Stack {
      base_addr.as_usize() - x * PAGE_SIZE
    } else {
      base_addr.as_usize() + x * PAGE_SIZE
    };
    trace!("unmap: {:#018x}", addr);
    apt.unmap_no_free(Page::containing_address(addr), pm);
  }
}

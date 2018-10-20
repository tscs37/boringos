use ::process_manager::ProcessHandle;
use ::vmem::PhysAddr;
use ::vmem::PAGE_SIZE;
use ::vmem::pagetable::Page;
use ::vmem::pagetable::{ActivePageTable,EntryFlags};
use ::alloc::vec::Vec;

pub enum MapType {
  Stack, // Stack Page, No Execute
  Data, // Data Page, No Execute
  Code, // Code Page, No Write
  Managed(ProcessHandle), // Memory available via other process
  ShMem(ProcessHandle), // Memory shared to other process
  Guard, // No Execute, No Read+Write
}

pub fn map(base_addr: PhysAddr, pl: Vec<PhysAddr>, mt: MapType) {
  let mut apt = unsafe { ActivePageTable::new() };
  let pm = &mut ::PAGER.lock();
  let flags = match mt {
    MapType::Stack => EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE,
    MapType::Data => EntryFlags::WRITABLE | EntryFlags::NO_EXECUTE,
    MapType::Code => EntryFlags::PRESENT,
    MapType::Managed(_) => EntryFlags::OS_EXTERNAL,
    MapType::ShMem(_) => EntryFlags::OS_EXTERNAL,
    MapType::Guard => EntryFlags::NO_EXECUTE,
  };
  for x in 0..pl.len() {
    let addr = base_addr.as_usize() + x * PAGE_SIZE;
    apt.map_to(Page::containing_address(addr), pl[x], flags, pm);
  }
}

pub fn unmap(base_addr: PhysAddr, pl: Vec<PhysAddr>) {
  panic!("unmap not supported yet")
}
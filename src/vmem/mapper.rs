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

pub fn map_new(base_addr: PhysAddr, mt:MapType) {
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
  apt.map(Page::containing_address(base_addr.as_usize()), flags, pm);
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
    let addr = base_addr.as_usize() - x * PAGE_SIZE;
    apt.map_to(Page::containing_address(addr), pl[x], flags, pm);
  };
  match mt {
    _ => (),
    Stack => {
      let guard_addr = unsafe { PhysAddr::new_unchecked(
        base_addr.as_u64() + PAGE_SIZE as u64) };
      let guard_addr_op = unsafe { PhysAddr::new_unchecked(
        base_addr.as_u64() - ((pl.len() + 1) * PAGE_SIZE) as u64)
      };
      let guard_map = unsafe { vec!(PhysAddr::new_unchecked(::vmem::GUARD_PAGE as u64)) };
      trace!("map stackunderflow guard page");
      map(guard_addr, guard_map.clone(), MapType::Guard);
      trace!("map stackoverflow guard page");
      map(guard_addr_op, guard_map.clone(), MapType::Guard);
    }
  };
}

pub fn unmap(base_addr: PhysAddr, pl_size: usize, mt: MapType) {
  let mut apt = unsafe { ActivePageTable::new() };
  let pm = &mut ::PAGER.lock();
  for x in 0..pl_size {
    let addr = base_addr.as_usize() - x * PAGE_SIZE;
    apt.unmap(Page::containing_address(addr), pm);
  }
  match mt {
    _ => (),
    Stack => {
      let guard_addr = unsafe { PhysAddr::new_unchecked(
        base_addr.as_u64() + PAGE_SIZE as u64) };
      let guard_addr_op = unsafe { PhysAddr::new_unchecked(
        base_addr.as_u64() - ((pl_size + 1) * PAGE_SIZE) as u64)
      };
      trace!("unmap stackunderflow guard page");
      unmap(guard_addr, 1, MapType::Guard);
      trace!("unmap stackoverflow guard page");
      unmap(guard_addr_op, 1, MapType::Guard);
    }
  }
}
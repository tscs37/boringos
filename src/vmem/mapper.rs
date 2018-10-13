use ::process_manager::ProcessHandle;
use ::vmem::PhysAddr;
use ::alloc::vec::Vec;

pub enum MapType {
  Stack,
  Data,
  Code,
  Managed(ProcessHandle),
  ShMem(ProcessHandle),
}

fn map(base_addr: PhysAddr, pl: Vec<PhysAddr>, mt: MapType) {

}

fn unmap(base_addr: PhysAddr, pl: Vec<PhysAddr>) {

}
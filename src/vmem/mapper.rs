use ::process_manager::ProcessHandle;
use ::vmem::PhysAddr;
use ::alloc::vec::Vec;

pub enum MapType {
  Stack, // Stack Page, No Execute
  Data, // Data Page, No Execute
  Code, // Code Page, No Write
  Managed(ProcessHandle), // Memory available via other process
  ShMem(ProcessHandle), // Memory shared to other process
  Guard, // No Execute, No Read+Write
}

fn map(base_addr: PhysAddr, pl: Vec<PhysAddr>, mt: MapType) {

}

fn unmap(base_addr: PhysAddr, pl: Vec<PhysAddr>) {

}
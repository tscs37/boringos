use super::{StateLoader, StateError};
use alloc::vec::Vec;
use alloc::boxed::Box;
use goblin::elf::Elf;
use goblin::elf::program_header::ProgramHeader;

pub struct ElfLoader<'a> {
  data: &'a [u8],
  elf: Elf<'a>,
}

impl<'a> ElfLoader<'a> {
  fn is_data(ph: &ProgramHeader) -> bool {
    let vmr = ph.vm_range();
    let sr = crate::vmem::DATA_START..crate::vmem::DATA_END;
    if sr.contains(&vmr.start) && sr.contains(&vmr.end) {
      return true;
    }
    return false;
  }
  fn is_code(ph: &ProgramHeader) -> bool {
    let vmr = ph.vm_range();
    let sr = crate::vmem::CODE_START..crate::vmem::CODE_END;
    if sr.contains(&vmr.start) && sr.contains(&vmr.end) {
      return true;
    }
    return false;
  }
}

impl<'a> StateLoader<'a> for ElfLoader<'a> {
  fn init(data: &'a [u8]) -> Result<Self, StateError> {
    let elf = match Elf::parse(data) {
      Ok(v) => v,
      Err(v) => { return Err(StateError::ELFParseError(v)) },
    };
    Ok(ElfLoader{ data, elf })
  }
  fn text(&self) -> Box<[u8]> {
    let mut data = Vec::new();
    let phs = self.elf.program_headers.clone();
    for ph in phs {
      if ElfLoader::is_code(&ph) {
        data.extend(&self.data[ph.file_range()]);
      }
    }
    data.into_boxed_slice()
  }
  fn data(&self) -> Box<[u8]> {
    let mut data = Vec::new();
    let phs = self.elf.program_headers.clone();
    for ph in phs {
      if ElfLoader::is_data(&ph) {
        data.extend(&self.data[ph.file_range()]);
      }
    }
    data.into_boxed_slice()
  }
  fn entry(&self) -> u64 {
    self.elf.entry
  }
}
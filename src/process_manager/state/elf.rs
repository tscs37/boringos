use super::{StateLoader, StateError, Section};
use alloc::vec::Vec;
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
  fn is_nonstd_code_start(ph: &ProgramHeader) -> bool {
    let vmr = ph.vm_range();
    let sr = crate::vmem::CODE_START;
    sr != vmr.start
  }
  fn is_nonstd_data_start(ph: &ProgramHeader) -> bool {
    let vmr = ph.vm_range();
    let sr = crate::vmem::DATA_START;
    sr != vmr.start
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
  fn text(&self) -> Section {
    let mut data = Vec::new();
    let phs = self.elf.program_headers.clone();
    let mut first = None;
    let mut last = 0;
    for ph in phs {
      if ElfLoader::is_code(&ph) {
        if first.is_none() && ElfLoader::is_nonstd_data_start(&ph) {
          first = Some(ph.file_range().start as u64);
        }
        if last < ph.file_range().end {
          let pad = ph.file_range().end - last;
          trace!("padding by {} bytes", pad);
          data.resize(data.len() + pad, 0);
        }
        last = ph.file_range().end;
        data.extend(&self.data[ph.file_range()]);
      }
    }
    Section::new(first, data.into_boxed_slice())
  }
  fn data(&self) -> Section {
    let mut data = Vec::new();
    let phs = self.elf.program_headers.clone();
    let mut first = None;
    let mut last = 0;
    for ph in phs {
      if ElfLoader::is_data(&ph) {
        if first.is_none() && ElfLoader::is_nonstd_data_start(&ph) {
          trace!("skipping first {:#010x} bytes", ph.file_range().start);
          first = Some(ph.file_range().start as u64);
        }
        if last < ph.file_range().end {
          let pad = ph.file_range().end - last;
          trace!("padding by {} bytes", pad);
          data.resize(data.len() + pad, 0);
        }
        last = ph.file_range().end;
        trace!("adding {:#010x} bytes", ph.file_range().len());
        data.extend(&self.data[ph.file_range()]);
      }
    }
    Section::new(first, data.into_boxed_slice())
  }
  fn entry(&self) -> u64 {
    self.elf.entry
  }
}
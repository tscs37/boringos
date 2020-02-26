use super::StateError;
use core::marker::Sized;
use alloc::boxed::Box;

pub struct Section {
  start: Option<u64>,
  data: Box<[u8]>,
}

pub trait StateLoader<'a> {
  fn init(data: &'a [u8]) -> Result<Self, StateError> where Self: Sized;
  // Return the text section as well as the offset
  fn text(&self) -> Section;
  fn data(&self) -> Section;
  fn entry(&self) -> u64;
}

impl Section {
  pub fn new(start: Option<u64>, data: Box<[u8]>) -> Self {
    Self{start, data}
  }
  fn pre_touch(base: u64, start: Option<u64>) {
    match start {
      None => (),
      Some(start) => {
        for ptr in 0..(start/4096)+1 {
          let addr = base + ptr * crate::vmem::PAGE_SIZE as u64;
          unsafe{core::ptr::read_volatile(addr as *mut u8)};
        }
      }
    }
  }
  pub fn load_at(self, base: u64) {
    let data = self.data;
    let size = data.len();
    let data = Box::into_raw(data);
    debug!("loading elf @ {:#018x} / +{:#010x} l{:#010x}", base, self.start.unwrap_or(0), size);
    trace!("pre-touching memory");
    Self::pre_touch(base, self.start);
    let base = base + self.start.unwrap_or(0);
    trace!("copying data to memory");
    unsafe {
      core::intrinsics::copy(
        data as *mut u8,
        base as *mut u8,
        size,
        )
    }
    trace!("copy completed, dropping buffer");
    drop(unsafe{Box::from_raw(data)});
  }
}
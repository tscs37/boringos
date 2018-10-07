

pub const PageSize: usize = 4096; // 4k FrameSize

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
  pub number: usize,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slab {
  pub address: usize,
  pub size: usize,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RawFrame {
  pub address: u64,
}

impl Frame {
  pub fn containing_address(address: usize) -> Frame {
    Frame { number: address / PageSize }
  }
  pub unsafe fn deref(&self) -> *mut u8 {
    (self.number * PageSize) as *mut u8
  }
  pub unsafe fn ro_deref(&self) -> *const u8 {
    (self.number * PageSize) as *const u8
  }
}

impl RawFrame {
  pub fn to_frame(&self) -> Frame {
    Frame{ number: (self.address as usize / PageSize) }
  }
}

pub trait FrameAllocator {
  fn allocate_frame(&mut self) -> Option<Frame>;
  fn deallocate_frame(&mut self, frame: Frame);
}

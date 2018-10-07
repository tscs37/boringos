
use vmem::fmem::{FrameAllocator,Frame,PageSize};

pub const BootMemorySize: usize = 256 * PageSize;

pub struct BitmapFrameAllocator {
  pub mem: [u8; BootMemorySize],
  pub allocbitmap: [u8; 4096],
}

impl BitmapFrameAllocator {
  fn ro_deref(&self) -> *const [u8; BootMemorySize] {
    &self.mem as *const [u8; BootMemorySize]
  }
  fn rw_deref(&mut self) -> *mut [u8; BootMemorySize] {
    &mut self.mem as *mut [u8; BootMemorySize]
  }
  fn bitmap_ro_deref(&self) -> *const [u8; 4096] {
    &self.allocbitmap as *const [u8; 4096]
  }
  fn bitmap_rw_deref(&mut self) -> *mut [u8; 4096] {
    &mut self.allocbitmap as *mut [u8; 4096]
  }
  fn frame_num_emulated(&self) -> usize {
    (&self.mem as *const [u8; BootMemorySize]) as usize / PageSize
  }
  fn framed_size(&self) -> usize {
    self.mem.len() / PageSize
  }
  pub fn has_free(&self) -> bool {
    for x in 0 .. self.framed_size()-1 {
      if !self.frame_in_use(x) {
        return true;
      }
    }
    false
  }

  fn frame_in_use(&self, index: usize) -> bool {
    if self.mem.len() / PageSize < index {
      panic!("attempt to reference frame outside frame bitmap")
    }
    let almap_bit_idx = index & 0x7;
    let (almap_real_idx, _) = index.overflowing_shr(3);
    let v = 0 != self.allocbitmap[almap_real_idx] & (0b1<<almap_bit_idx);
    v
  }

  fn use_frame(&mut self, index:usize) {
    if self.mem.len() / PageSize < index {
      panic!("attempt to reference frame outside frame bitmap")
    }
    let almap_bit_idx = index & 0x7;
    let (almap_real_idx, _) = index.overflowing_shr(3);
    self.allocbitmap[almap_real_idx] = self.allocbitmap[almap_real_idx] | (0b1<<almap_bit_idx);
  }

  fn free_frame(&mut self, index:usize) {
    //let almap = self.bitmap_rw_deref();
    if self.mem.len() / PageSize < index {
      panic!("attempt to reference frame outside frame bitmap")
    }
    let almap_bit_idx = index & 0x7;
    let (almap_real_idx, _) = index.overflowing_shr(3);
    self.allocbitmap[almap_real_idx] = self.allocbitmap[almap_real_idx] & !(0b1<<almap_bit_idx);
  }
}

impl FrameAllocator for BitmapFrameAllocator {
  fn allocate_frame(&mut self) -> Option<Frame> {
    for x in 0..self.mem.len() {
      if !self.frame_in_use(x) {
        self.use_frame(x);
        let frame_base = (&mut self.mem[x * PageSize]) as *mut u8;
        let frame = Frame::containing_address(frame_base as usize);
        return Some(frame);
      }
    }
    None
  }

  fn deallocate_frame(&mut self, frame: Frame) {
    self.free_frame(frame.number)
  }
}

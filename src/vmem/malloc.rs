use core::alloc::GlobalAlloc;
use core::alloc::{AllocErr, Layout};
use core::ptr::NonNull;
use spin::Mutex;
use vmem::bmfa::BitmapFrameAllocator;
use vmem::fmem::PageSize;

use alloc::rc::Rc;
use core::cell::RefCell;

unsafe impl<'a> GlobalAlloc for Allocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let q = (*self).internal_alloc(layout);
    if q.is_err() {
      return 0 as *mut u8;
    }
    q.unwrap().as_ptr()
  }
  unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
    (*self).internal_dealloc(NonNull::new(ptr).unwrap(), layout)
  }
}

use alloc::boxed::Box;
use vmem::slab::SlabCollection;

pub struct Allocator {
  pub boot_memory: Mutex<BitmapFrameAllocator>,
  pub slab_memory: Option<Rc<RefCell<SlabCollection>>>,
  pub disable_boot_memory: bool,
}

impl Allocator {
  fn has_more(&self) -> bool {
    match self.slab_memory {
      Some(ref sc) => {
        let scp = (*sc).try_borrow();
        if scp.is_err() {
          if self.disable_boot_memory {
            panic!(
              "could not check free memory: {}; boot_memory disabled",
              scp.err().unwrap()
            );
          } else {
            debug!(
              "could not check free memory: {}; fallback to boot_memory",
              scp.err().unwrap()
            );
          }
          return self.boot_memory.lock().has_free();
        }
        return scp.unwrap().has_free();
      }
      None => self.boot_memory.lock().has_free(),
    }
  }

  pub unsafe fn disable_boot_memory(&mut self) {
    if self.disable_boot_memory { panic!("disable_boot_memory called twice"); }
    match self.slab_memory {
      Some(ref sc) => {
        match (*sc).try_borrow() {
          Ok(scp) => {
            scp.init_steal_boot(&self.boot_memory);
            scp.init_steal_first();
          },
          Err(e) => panic!("error during initial slab steal: {}", e)
        }
      },
      None => panic!("disable_boot_memory needs slab memory to be intialized")
    }
    self.disable_boot_memory = true;
  }

  unsafe fn internal_alloc(&self, layout: Layout) -> Result<NonNull<u8>, AllocErr> {
    use vmem::fmem::FrameAllocator;

    if !self.has_more() {
      return Err(AllocErr {});
    }

    if self.disable_boot_memory {
      debug!("boot memory disabled, using slab memory");
      match self.slab_memory {
        Some(ref sm) => {
          let smbr = sm.try_borrow();
          match smbr {
            Ok(ref smb) => {
              let sao = smb.select_sa(layout.size());
              match sao {
                Some(sa) => {
                  use vmem::slab::SizedAllocator;
                  let mut sam = super::slab::conv_spr(&sa);
                  match sam.allocate_slab() {
                    Some(slab) => {
                      let nnptr = NonNull::new(slab.address()).unwrap();
                      debug!("PTR={:?}", nnptr);
                      return Ok(nnptr);
                    }
                    None => {
                      panic!("Could not allocate slab");
                    }
                  }
                }
                None => {
                  panic!("no slab allocator found");
                }
              }
            }
            Err(e) => {
              panic!("could not borrow slab memory: {}", e);
            }
          }
        }
        None => {
          panic!("boot_memory disabled but slab memory uninitialized");
        }
      }
    } else {
      if layout.size() > PageSize {
        return Err(AllocErr {});
      }

      if let Some(frame) = self.boot_memory.lock().allocate_frame() {
        let nnptr = NonNull::new(frame.deref()).unwrap();
        return Ok(nnptr);
      }
      panic!("internal memory allocator returned none")
    }
  }
  unsafe fn internal_dealloc(&self, ptr: NonNull<u8>, layout: Layout) {
    debug!("deallocating {:#016x}", ptr.as_ptr() as usize);
    if self.disable_boot_memory {
      match self.slab_memory {
        Some(ref sm) => {
          let smbr = sm.try_borrow();
          match smbr {
            Ok(ref smb) => {
              let sao = smb.select_sa(layout.size());
              match sao {
                Some(sa) => {
                  use vmem::slab::SizedAllocator;
                  let mut sam = super::slab::conv_spr(&sa);
                  sam.deallocate_slab(super::slab::Slab::from_ptr(ptr, layout));
                }
                None => {
                  panic!("no slab allocator found");
                }
              }
            }
            Err(e) => {
              panic!("could not borrow slab memory: {}", e);
            }
          }
        }
        None => {
          panic!("boot_memory disabled but slab memory uninitialized");
        }
      }
    } else {
      debug!("ignoring dealloc")
    }
  }

  pub unsafe fn init_slab_memory(&mut self) {
    debug!("Allocating slab memory manager...");
    self.slab_memory = Some(Rc::new(RefCell::new(SlabCollection::new())));
  }

  pub unsafe fn push_slab<'a>(&mut self, addr: usize, size: usize) -> Result<usize, &'a str> {
    match self.slab_memory {
      Some(ref sc) => {
        let scp = (*sc).try_borrow_mut();
        match scp {
          Ok(scp_mut) => {
            use vmem::slab::SizedAllocator;
            return scp_mut.push_slab(addr, size);
          }
          Err(e) => { panic!("could not borrow slab allocator mutably: {}", e); }
        }
      }
      None => {
        panic!("Attempted to slab none");
      }
    }
  }

  pub unsafe fn free_memory<'a>(&self) -> Result<usize, &'a str> {
        match self.slab_memory {
      Some(ref sc) => {
        let scp = (*sc).try_borrow();
        match scp {
          Ok(scpb) => {
            use vmem::slab::SizedAllocator;
            return Ok(scpb.count_free())
          }
          Err(e) => { panic!("could not borrow slab allocator mutably: {}", e); }
        }
      }
      None => {
        return Ok(super::bmfa::BootMemorySize);
      }
    }
  }
}

/// Align downwards. Returns the greatest x with alignment `align`
/// so that x <= addr. The alignment must be a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
  if align.is_power_of_two() {
    addr & !(align - 1)
  } else if align == 0 {
    addr
  } else {
    panic!("`align` must be a power of 2");
  }
}

/// Align upwards. Returns the smallest x with alignment `align`
/// so that x >= addr. The alignment must be a power of 2.
pub fn align_up(addr: usize, align: usize) -> usize {
  align_down(addr + align - 1, align)
}

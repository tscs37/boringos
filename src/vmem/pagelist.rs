mod pagelist_og;
mod pagelist_ng;

pub use crate::vmem::pagelist::pagelist_og::*;

use core::ptr::NonNull;
use core::cmp::Ordering;

// we ignore address 0
#[derive(Clone,Copy)]
pub struct PhysAddr(NonNull<u8>);

pub type RelativeFrame = usize;

assert_eq_size!(check_phys_addr_size; PhysAddr,    u64);

pub enum PagePoolAllocationError {
  /// No free page found
  NoPageFree,
}

pub enum PagePoolReleaseError {
  /// Page has been released already (double-free)
  PageAlreadyUnused,
  /// Page is not tracked in free memory
  PageUntracked,
}

pub enum PagePoolAppendError {
  /// The supplied amount of pages for bootstrapping was not enough
  NotEnoughMemory,
  /// Returned when a None? is run
  NoneError(core::option::NoneError),
  /// Returned when the append operation could not allocate
  AllocError(core::alloc::AllocErr),
}

impl From<core::option::NoneError> for PagePoolAppendError {
  fn from(e: core::option::NoneError) -> Self { PagePoolAppendError::NoneError(e) }
}

impl From<core::alloc::AllocErr> for PagePoolAppendError {
  fn from(e: core::alloc::AllocErr) -> Self { PagePoolAppendError::AllocError(e) }
}


/// This trait contains all necessary functions the kernel operates on 
/// The following properties must be satified by the page pool mechanism:
///   * All functions except init() and dump() must be reentrant
///   * init() must fail safely and clear the memory pages it dirtied
///   * If allocation is required, the pagepool should use the normal allocator
///   * The allocator must provide a method to initialize the page pool using pre-allocated memory
/// 
/// Care must be taken when adding memory; this operation is not atomic and the page pool
/// does not guarantee it will work while the memory is being added.
pub trait PagePool: Sized {

  /// Returns true if there are free memory pages.
  /// This value is non-authorative; a call to allocate() can still fail.
  fn has_free(&self) -> bool;

  /// Returns the number of free memory pages, this value should be cached
  fn count_free(&self) -> usize;

  /// Outputs the page pool into the kernel debug log
  fn dump(&self);

  /// This function will allocate a memory page from it's internal pool if possible.
  /// If there is no memory available in the pool, None is returned. 
  /// The returned memory page must be zeroed.
  fn allocate(&mut self) -> Result<PhysAddr, PagePoolAllocationError>;
  /// Releases a memory page to be reused. If the page is pinned, a non-fatal error
  /// must be returned.
  fn release(&mut self, pa: PhysAddr) -> Result<(), PagePoolReleaseError>;

  /// A section of memory specified by pa and sz is to be added to the page pool.
  /// The page pool must use the normal memory allocator for this operation.
  /// THIS OPERATION IS NOT REENTRANT OR ATOMIC
  fn add_memory(&mut self, pa: PhysAddr, sz: usize) -> Result<(), PagePoolAppendError>;
}

impl ::core::fmt::Display for PhysAddr {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    f.write_fmt(format_args!("0x{:016X}", self.as_u64()))
  }
}

impl ::core::fmt::Debug for PhysAddr {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    f.write_fmt(format_args!("0x{:016X}", self.as_u64()))
  }
}

impl ::core::ops::Add<usize> for PhysAddr {
  type Output = PhysAddr;

  fn add(self, rhs: usize) -> PhysAddr {
    let lhs = self.as_usize();
    PhysAddr::new_usize_or_abort(lhs.saturating_add(rhs))
  }
}

// subtracting from a physaddr with a physaddr results in a relative frame
impl ::core::ops::Sub<PhysAddr> for PhysAddr {
  type Output = RelativeFrame;

  fn sub(self, rhs: PhysAddr) -> RelativeFrame {
    (self.as_usize() - rhs.as_usize()) / crate::vmem::PAGE_SIZE
  }
}

impl ::core::cmp::PartialEq for PhysAddr {
  fn eq(&self, rhs: &PhysAddr) -> bool {
    self.as_u64() == rhs.as_u64()
  }
}

impl ::core::cmp::PartialOrd for PhysAddr {
  fn partial_cmp(&self, rhs: &PhysAddr) -> Option<Ordering> {
    self.as_u64().partial_cmp(&rhs.as_u64())
  }
}
impl PhysAddr {
  pub fn new(p: u64) -> Option<PhysAddr> {
    assert!(p < 0x0000_8000_0000_0000 ||
      p >= 0xffff_8000_0000_0000,
      "invalid address: {:#018x}", p);
    match NonNull::new(p as *mut u8) {
      Some(nn) => Some(PhysAddr(nn)),
      None => None
    }
  }
  pub fn new_usize(p: usize) -> Option<PhysAddr> {
    PhysAddr::new(p as u64)
  }
  pub fn from(nn: NonNull<u8>) -> PhysAddr {
    assert!((nn.as_ptr() as usize) < 0x0000_8000_0000_0000 ||
      (nn.as_ptr() as usize) >= 0xffff_8000_0000_0000,
      "invalid address: {:#018x}", (nn.as_ptr() as usize));
    PhysAddr(nn)
  }
  pub fn into<T>(self) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(self.0.as_ptr() as *mut T) }
  }
  pub unsafe fn new_unchecked(p: u64) -> PhysAddr {
    assert!(p < 0x0000_8000_0000_0000 ||
      p >= 0xffff_8000_0000_0000,
      "invalid address: {:#018x}", p);
    PhysAddr(NonNull::new_unchecked(p as *mut u8))
  }
  pub fn new_or_abort(p: u64) -> PhysAddr {
    match PhysAddr::new(p) {
      None => panic!("could not create physaddr for {:#018x}, probably was null or illegal", p),
      Some(pa) => pa
    }
  }
  pub fn new_usize_or_abort(p: usize) -> PhysAddr {
    PhysAddr::new_or_abort(p as u64)
  }
  // Adds the specified number of pages as offset and returns the result as PhysAddr
  pub unsafe fn add_pages(&self, pages: u64) -> PhysAddr {
    PhysAddr(NonNull::new_unchecked(
      (self.as_u64() + (pages * crate::vmem::PAGE_SIZE as u64)) as *mut u8))
  }
  pub fn as_u64(&self) -> u64 {
    self.as_mut8() as u64
  }
  pub fn as_usize(&self) -> usize {
    self.as_u64() as usize
  }
  pub fn as_mut8(&self) -> *mut u8 {
    self.0.as_ptr()
  }
  pub fn as_ptr<T>(&self) -> *mut T {
    self.0.as_ptr() as *mut T
  }
  pub fn as_physaddr(&self) -> ::x86_64::PhysAddr {
    ::x86_64::PhysAddr::new(self.0.as_ptr() as u64)
  }
}

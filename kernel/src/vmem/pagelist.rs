
pub mod pagelist_ng;

pub use crate::vmem::pagelist::pagelist_ng::*;

use core::alloc::AllocErr;
use x86_64::PhysAddr;
use x86_64::structures::paging::{PhysFrame, UnusedPhysFrame};
use core::option::NoneError;

pub type RelativeFrame = usize;

#[derive(Debug, Clone)]
pub enum PagePoolAllocationError {
  /// No free page found
  NoPageFree,
}

#[derive(Debug, Clone)]
pub enum PagePoolReleaseError {
  /// Page has been released already (double-free)
  PageAlreadyUnused,
  /// Page is not tracked in free memory
  PageUntracked,
}

#[derive(Debug, Clone)]
pub enum PagePoolAppendError {
  /// The supplied amount of pages for bootstrapping was not enough
  NotEnoughMemory,
  /// Returned when a None? is run
  NoneError(NoneError),
  /// Returned when the append operation could not allocate
  AllocError(AllocErr),
}

impl From<AllocErr> for PagePoolAppendError {
  fn from(e: AllocErr) -> Self { PagePoolAppendError::AllocError(e) }
}

impl From<NoneError> for PagePoolAppendError {
  fn from(e: NoneError) -> Self { PagePoolAppendError::NoneError(e) }
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
pub trait PagePool {

  /// Returns true if there are free memory pages.
  /// This value is non-authorative; a call to allocate() can still fail.
  fn has_free(&self) -> bool;

  /// Returns the number of free memory pages, this value should be cached
  fn count_free(&self) -> usize;

  /// Returns the total number of memory pages in the page pool
  fn count_all(&self) -> usize;

  fn count_used(&self) -> usize {
    self.count_all() - self.count_free()
  } 

  /// Outputs the page pool into the kernel debug log
  fn dump(&self);

  /// This function will allocate a memory page from it's internal pool if possible.
  /// If there is no memory available in the pool, None is returned. 
  /// The returned memory page must be zeroed.
  fn allocate(&mut self) -> Result<UnusedPhysFrame, PagePoolAllocationError>;
  /// Releases a memory page to be reused. If the page is pinned, a non-fatal error
  /// must be returned.
  fn release(&mut self, pa: PhysFrame) -> Result<(), PagePoolReleaseError>;

  /// A section of memory specified by pa and sz is to be added to the page pool.
  /// The page pool must use the normal memory allocator for this operation.
  /// THIS OPERATION IS NOT REENTRANT OR ATOMIC
  /// The caller must provide a valid allocation to this function
  /// as allocating memory is not possible inside
  fn add_memory(&mut self, alloc: *mut PageMap, pa: PhysAddr, sz: u64) -> Result<u64, PagePoolAppendError>;
}

use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, Size4KiB};

unsafe impl FrameAllocator<Size4KiB> for dyn PagePool {
  fn allocate_frame(&mut self) -> Option<UnusedPhysFrame> {
    debug!("frame allocation request");
    let alloc = self.allocate();
    match alloc {
      Err(v) => { debug!("could not allocate frame: {:?}", v); None },
      Ok(alloc) => { debug!("allocated frame {:#018x}", alloc.start_address()); Some(alloc) }
    }
  }
}

impl FrameDeallocator<Size4KiB> for dyn PagePool  {
  fn deallocate_frame(&mut self, frame: UnusedPhysFrame) {
    self.release(*frame).unwrap()
  }
}
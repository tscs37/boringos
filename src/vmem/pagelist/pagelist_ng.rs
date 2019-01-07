use crate::vmem::pagelist::{PhysAddr, PagePool, PagePoolAllocationError, PagePoolReleaseError};
use crate::vmem::pagelist::PagePoolAppendError;
use core::ptr::NonNull;
use core::option::NoneError;
use core::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use core::convert::TryFrom;
use crate::vmem::PAGE_SIZE;

const PAGES_PER_BLOCK: usize = 4076;

#[repr(align(4096))]
pub struct PageMap {
  start: PhysAddr,
  size: u16,
  next: Option<PageMapWrapper>,
  free_pages: AtomicU16,
  used: [AtomicBool; PAGES_PER_BLOCK],
}

panic_on_drop!(PageMap);

impl PageMap {
  /// Allocates a PageMap without relying on the alloc:: crate yet, useful
  /// when initializing the pagemapper, this will consume some memory
  pub fn new_no_alloc(start: PhysAddr, size: u16) -> Result<*mut PageMap, PagePoolAllocationError> {
    assert!(size as usize <= PAGES_PER_BLOCK, "must not specify more than PAGES_PER_BLOCK for on-stack pagemap");
    trace!("allocating pagemap on stack");
    let mut page_map = PageMap {
      start,
      size,
      next: None,
      free_pages: AtomicU16::new(size),
      used: unsafe{core::mem::uninitialized()},
    };
    for x in 0..PAGES_PER_BLOCK {
      page_map.used[x] = AtomicBool::new(false);
    }
    trace!("allocating memory from pagemap on stack");
    let mut pmw: PageMapWrapper = PageMapWrapper::from(&mut page_map);
    let self_alloc = pmw.allocate()?;
    let self_alloc = self_alloc.as_ptr() as *mut PageMap;
    drop(pmw);
    trace!("lift pagemap into memory {:#018x}", self_alloc as u64);
    unsafe { page_map.move_into(self_alloc) };
    Ok(self_alloc)
  }
  /// copies the pagemap into a memory location and forgets about the result
  unsafe fn move_into(self, pm: *mut PageMap) {
    (*pm).start = self.start;
    (*pm).size = self.size;
    (*pm).next = self.next;
    (*pm).free_pages = AtomicU16::new(self.free_pages.load(Ordering::SeqCst));
    (*pm).used = core::mem::uninitialized();
    for x in 0..PAGES_PER_BLOCK {
      (*pm).used[x] = AtomicBool::new(self.used[x].load(Ordering::SeqCst));
    }
    core::mem::forget(self);
  }
  // creates a new pagemap at the indicates position
  // and consumes as many pages as possible until either it's capacity
  // is exhausted or PAGES_PER_BLOCK maximum is reached.
  // If PAGES_PER_BLOCK was exhausted, the remaining pages are returned.
  fn new(page_map: *mut PageMap, start: PhysAddr, size: u16) -> (*mut PageMap, u16) {
    assert_eq!(PAGES_PER_BLOCK & 0xFFFF, PAGES_PER_BLOCK, "PAGES_PER_BLOCK must fit in u16");
    let actual_size: u16 = core::cmp::min(size, PAGES_PER_BLOCK as u16);
    let rem_size = if actual_size != size { size.saturating_sub(actual_size) } else { 0 };
    unsafe {(*page_map) = PageMap {
      start,
      size: actual_size,
      free_pages: AtomicU16::new(size),
      next: None,
      used: core::mem::uninitialized(),
    }};
    for x in 0..PAGES_PER_BLOCK {
      unsafe { (*page_map).used[x] = AtomicBool::new(false) };
    }
    (page_map, rem_size)
  }
}

#[derive(Copy, Clone)]
pub struct PageMapWrapper(NonNull<PageMap>);

impl core::ops::Deref for PageMapWrapper {
  type Target = PageMap;

  fn deref(&self) -> &PageMap {
    unsafe{&(*self.0.as_ptr())}
  }
}

impl core::ops::DerefMut for PageMapWrapper {
  fn deref_mut(&mut self) -> &mut PageMap {
    unsafe{&mut (*self.0.as_ptr())}
  }
}

impl core::fmt::Debug for PageMapWrapper {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(f, "{:#018x}", self.0.as_ptr() as usize)
  }
}

impl From<&mut PageMap> for PageMapWrapper {
  fn from(pm: &mut PageMap) -> PageMapWrapper {
    PageMapWrapper(NonNull::from(pm))
  }
}

impl TryFrom<*mut PageMap> for PageMapWrapper {
  type Error = NoneError;

  fn try_from(pm: *mut PageMap) -> Result<PageMapWrapper, NoneError> {
    Ok(PageMapWrapper(NonNull::new(pm)?))
  }
}

impl core::fmt::Display for PageMapWrapper {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl PagePool for PageMapWrapper {

  fn has_free(&self) -> bool {
    self.free_pages.load(Ordering::Relaxed) > 0 || match self.next {
      Some(next) => next.has_free(),
      None => false,
    }
  }

  fn count_free(&self) -> usize {
    self.free_pages.load(Ordering::Relaxed) as usize + match self.next {
      Some(next) => next.count_free(),
      None => 0,
    }
  }

  fn dump(&self) {
    debug!("PagePool for {}", self.start);
    debug!("Size: {} pages, {} KB", self.size, self.size * 4);
    debug!("Free Pages: {}", self.free_pages.load(Ordering::Relaxed));
    debug!("Next PagePool: {:?}", self.next);
  }

  fn allocate(&mut self) -> Result<PhysAddr, PagePoolAllocationError> {
    trace!("allocating page into memory");
    for x in 0..PAGES_PER_BLOCK {
      let prev = self.used[x].compare_and_swap(false, true, Ordering::SeqCst);
      if !prev {
        let addr = self.start + (x * PAGE_SIZE);
        trace!("got page {}", addr);
        self.free_pages.fetch_sub(1, Ordering::SeqCst);
        return Ok(addr);
      }
    }
    trace!("no page found, trying next block");
    match self.next {
      Some(mut next) => next.allocate(),
      None => Err(PagePoolAllocationError::NoPageFree),
    }
  }

  fn release(&mut self, pa: PhysAddr) -> Result<(),PagePoolReleaseError> {
    if pa > (self.start + self.size as usize) {
      return Err(PagePoolReleaseError::PageUntracked)
    }
    let index = pa - self.start;
    let prev = self.used[index].compare_and_swap(true, false, Ordering::SeqCst);
    if prev {
      self.free_pages.fetch_add(1, Ordering::SeqCst);
      Ok(())
    } else {
      match self.next {
        Some(mut next) => next.release(pa),
        None => Err(PagePoolReleaseError::PageAlreadyUnused),
      }
    }
  }

  fn add_memory(&mut self, pa: PhysAddr, sz: usize) -> Result<(), PagePoolAppendError> {
    match self.next {
      Some(mut next) => next.add_memory(pa, sz),
      None => {
        use core::alloc::{Layout, Alloc};
        use alloc::alloc::Global;
        use core::cmp::min;

        // Allocate the pagemap in memory
        let layout = Layout::new::<PageMap>();
        assert!(layout.align() == PAGE_SIZE, "pages must be aligned to pagesize");
        let ptr: *mut PageMap = unsafe{Global{}.alloc_zeroed(layout)?}.cast().as_ptr();

        // Calculate maximum of pages we can put into the new map
        let sza = min(sz, core::u16::MAX as usize) as u16;

        // Create the new pagemap from memory
        let (pm, rem) = PageMap::new(ptr, pa, sza);
        let sza = sza as usize;
        let sza = sza + rem as usize;

        // Wrap the resulting pagemap
        let mut pm = PageMapWrapper(NonNull::new(ptr)?);

        // Update linked list
        self.next = Some(pm);
        if sz > sza {
          Ok(pm.add_memory(pa + sza, sz - sza)?)
        } else {
          Ok(())
        }
      }
    }
  }

}

//TODO: implement debug for PageMap

assert_eq_size!(check_phys_addr_size; PhysAddr,                 u64);
assert_eq_size!(check_page_map_size; PageMap,                   [u8; 4096]);
assert_eq_size!(check_page_map_wrapper; PageMapWrapper,         u64);
assert_eq_size!(check_page_map_wrapopt; Option<PageMapWrapper>, u64);

#[cfg(test)]
mod test {
//TODO: implement some tests
}
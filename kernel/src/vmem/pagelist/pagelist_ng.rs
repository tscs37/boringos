use crate::vmem::pagelist::{PhysAddr, PagePool, PagePoolAllocationError, PagePoolReleaseError};
use crate::vmem::pagelist::PagePoolAppendError;
use core::ptr::NonNull;
use core::option::NoneError;
use core::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use core::convert::TryFrom;
use crate::vmem::{PAGE_SIZE, KHEAP_START};
use crate::*;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::Size4KiB;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator};
use x86_64::structures::paging::UnusedPhysFrame;

const PAGES_PER_BLOCK: usize = 4067;
const HEADER_MAGIC: u64 = 0xDEADC0FFEE;

#[repr(align(4096))]
pub struct PageMap {
  header: u64,
  start: PhysAddr,
  size: u16,
  next: Option<PageMapWrapper>,
  free_pages: AtomicU16,
  /// If set, the pagetable lock function is disabled
  disable_pt_lock: bool,
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
      header: HEADER_MAGIC,
      start,
      size,
      next: None,
      disable_pt_lock: true,
      free_pages: AtomicU16::new(size),
      used: unsafe{core::mem::MaybeUninit::zeroed().assume_init()},
    };
    for x in 0..PAGES_PER_BLOCK {
      page_map.used[x] = AtomicBool::new(false);
    }

    let mut pmw = PageMapWrapper(NonNull::new(&mut page_map as *mut PageMap).unwrap());
    let page = pmw.allocate()?;
    drop(PageMapWrapper);

    trace!("installing temporary pagepool");
    unsafe{pager().overwrite_pagepool(pmw)};

    trace!("mapping page pool heap address");
    let alloc = KHEAP_START + 1 * PAGE_SIZE;
    let alloc = VirtAddr::new(alloc.try_into().unwrap());
    crate::vmem::mapper::map(alloc, &[page.start_address()], crate::vmem::mapper::MapType::Data);

    assert!(crate::vmem::mapper::is_mapped(alloc), "didn't map memory address");

    page_map.verify();
    

    let layout = alloc::alloc::Layout::for_value(&page_map);
    let alloc: *mut PageMap = alloc.as_mut_ptr();
    info!("lift pagemap into memory {:#018x}", alloc as u64);
    unsafe{core::ptr::write_volatile(alloc, page_map)};

    trace!("locking pagemap");
    unsafe{alloc.as_mut().unwrap().disable_pt_lock = false};
    unsafe{alloc.as_mut().unwrap().lock()};

    Ok(alloc)
  }
  // creates a new pagemap at the indicates position
  // and consumes as many pages as possible until either it's capacity
  // is exhausted or PAGES_PER_BLOCK maximum is reached.
  // If PAGES_PER_BLOCK was exhausted, the remaining pages are returned.
  fn new(page_map: *mut PageMap, start: PhysAddr, size: u64) -> (*mut PageMap, u64) {
    assert_eq!(PAGES_PER_BLOCK & 0xFFFF, PAGES_PER_BLOCK, "PAGES_PER_BLOCK must fit in u16");
    assert!(page_range!(KHEAP).contains(&VirtAddr::from_ptr(page_map)), "PageMap allocation outside kernel heap");
    info!("new page map @ {:?}", VirtAddr::from_ptr(page_map));
    trace!("calculating pagemap size");
    let actual_size: u16 = if size < PAGES_PER_BLOCK.try_into().unwrap() { size.try_into().unwrap() } 
      else { PAGES_PER_BLOCK.try_into().unwrap() };
    let rem_size: u64 = size - actual_size as u64;
    trace!("size = {}, actual_size = {}, rem_size = {}", size, actual_size, rem_size);
    trace!("writing to pagemap");
    let pm = PageMap {
      header: HEADER_MAGIC,
      start,
      size: actual_size,
      free_pages: AtomicU16::new(actual_size),
      disable_pt_lock: false,
      next: None,
      used: unsafe{core::mem::MaybeUninit::zeroed().assume_init()},
    };
    unsafe{core::ptr::write_volatile(page_map, pm)};
    trace!("clearing pagemap used bitmap");
    for x in 0..PAGES_PER_BLOCK {
      unsafe { (*page_map).used[x] = AtomicBool::new(false) };
    }
    unsafe{page_map.as_ref()}.map(|y| y.lock());
    (page_map, size - (actual_size as u64))
  }

  fn verify(&self) {
    assert_eq!(self.header, HEADER_MAGIC, "Header magic corrupted");
  }

  fn unlock(&self) {
    if !self.disable_pt_lock {
      let vaddr = VirtAddr::from_ptr(self as *const PageMap);
      crate::vmem::mapper::update_flags(vaddr, crate::vmem::mapper::MapType::Data);
    }
  }

  fn lock(&self) {
    if !self.disable_pt_lock {
      let vaddr = VirtAddr::from_ptr(self as *const PageMap);
      crate::vmem::mapper::update_flags(vaddr, crate::vmem::mapper::MapType::ReadOnly);
    }
  }
}

#[derive(Copy, Clone)]
pub struct PageMapWrapper(NonNull<PageMap>);

impl PageMapWrapper {
  fn verify(&self) {
    let pm: &PageMap = self;
    pm.verify();
  }
}

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
    assert!(page_range!(KHEAP).contains(&VirtAddr::from_ptr(pm as *const PageMap)), "PageMap allocation outside kernel heap");
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
    let val = self.free_pages.load(Ordering::Relaxed) as usize + match self.next {
      Some(next) => next.count_free(),
      None => 0,
    };
    val
  }

  fn count_all(&self) -> usize {
    (self.size as usize) + match self.next{
      Some(next) => next.count_all(),
      None => 0,
    } 
  } 

  fn dump(&self) {
    debug!("PagePool for {:?}", self.start);
    debug!("Size: {} pages, {} KB", self.size, self.size * 4);
    debug!("Free Pages: {}", self.free_pages.load(Ordering::Relaxed));
    debug!("Next PagePool: {:?}", self.next);
  }

  fn allocate(&mut self) -> Result<UnusedPhysFrame, PagePoolAllocationError> {
    self.verify();
    self.unlock();
    for x in 0..self.size {
      let x = x as usize;
      let prev = self.used[x].compare_and_swap(false, true, Ordering::SeqCst);
      if !prev {
        let addr = self.start + (x * PAGE_SIZE);
        trace!("free page from {:#018x} + {:#010x} = {:#018x}", 
          self.start.as_u64(),
          (x*PAGE_SIZE), addr);
        self.free_pages.fetch_sub(1, Ordering::SeqCst);
        self.lock();
        let addr = unsafe{UnusedPhysFrame::new(PhysFrame::
          from_start_address(addr).expect("allocated unaligned physical address"))};
        return Ok(addr);
      }
    }
    self.lock();
    trace!("no page found, trying next block");
    match self.next {
      Some(mut next) => next.allocate(),
      None => Err(PagePoolAllocationError::NoPageFree),
    }
  }

  fn release(&mut self, pa: PhysFrame<Size4KiB>) -> Result<(),PagePoolReleaseError> {
    self.verify();
    trace!("releasing memory {:?}", pa);
    if pa.start_address() > (self.start + self.size as usize) {
      return Err(PagePoolReleaseError::PageUntracked)
    }
    self.unlock();
    let index = (pa.start_address().as_u64() - self.start.as_u64()) as usize;
    let index = index / pa.size() as usize;
    trace!("index rel to pagelist is {}", index);
    let prev = self.used[index].compare_and_swap(true, false, Ordering::SeqCst);
    if prev {
      self.free_pages.fetch_add(1, Ordering::SeqCst);
      self.lock();
      Ok(())
    } else {
      self.lock();
      match self.next {
        Some(mut next) => next.release(pa),
        None => Err(PagePoolReleaseError::PageAlreadyUnused),
      }
    }
  }

  // Adds memory to pool and returns number of remaining pages
  fn add_memory(&mut self, alloc: *mut PageMap, pa: PhysAddr, sz: u64) -> Result<u64, PagePoolAppendError> {
    match self.next {
      Some(mut next) => next.add_memory(alloc, pa, sz),
      None => {
        trace!("adding {:?}+{} to page pool", pa, sz);

        // Create the new pagemap from memory
        let (pm, rem) = PageMap::new(alloc, pa, sz.into());
        trace!("rem: {}", rem);

        // Wrap the resulting pagemap
        let pm = PageMapWrapper(NonNull::new(alloc)?);
        trace!("PMW : {}", pm);

        // Update linked list
        self.unlock();
        self.next = Some(pm);
        self.lock();
        if sz-rem > 0 {
          trace!("added {} pages", sz-rem);
          Ok(sz-rem)
        } else {
          Ok(0)
        }
      }
    }
  }
}


unsafe impl FrameAllocator<Size4KiB> for PageMapWrapper  {
  fn allocate_frame(&mut self) -> Option<PhysFrame> {
    trace!("allocating frame from pagemapper");
    let pframe = self.allocate().unwrap();
    trace!("free frames remaining: {}", self.count_free());
    Some(*pframe)
  }
}

impl FrameDeallocator<Size4KiB> for PageMapWrapper {
  unsafe fn deallocate_frame(&mut self, frame: PhysFrame) {
    trace!("deallocating frame from pagemapper");
    self.release(frame).unwrap()
  }
}

assert_eq_size!(check_phys_addr_size; PhysAddr,                 u64);
assert_eq_size!(check_page_map_size; PageMap,                   [u8; 4096]);
assert_eq_size!(check_page_map_wrapper; PageMapWrapper,         u64);
assert_eq_size!(check_page_map_wrapopt; Option<PageMapWrapper>, u64);

#[cfg(test)]
mod test {
//TODO: implement some tests
}
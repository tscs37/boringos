

const PagesPerBlock: usize = 449; // Readjust this when struct layout changes
use vmem::PageSize;
use ::core::ptr::NonNull;

// we ignore address 0
pub struct PhysAddr(NonNull<u8>);

impl ::core::fmt::Display for PhysAddr {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    let string = format!("{:#018x}", self.as_u64());
    f.pad_integral(true, "", &string)
  }
}

impl ::core::fmt::Debug for PhysAddr {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    let string = format!("{:#018x}", self.as_u64());
    f.pad_integral(true, "", &string)
  }
}

impl PhysAddr {
  pub fn new(p: u64) -> Option<PhysAddr> {
    match NonNull::new(p as *mut u8) {
      Some(nn) => Some(PhysAddr(nn)),
      None => None
    }
  }
  pub unsafe fn new_unchecked(p: u64) -> PhysAddr {
    PhysAddr(NonNull::new_unchecked(p as *mut u8))
  }
  pub fn as_u64(&self) -> u64 {
    self.as_mut8() as u64
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

impl ::core::cmp::PartialEq for PhysAddr {
  fn eq(&self, rhs: &PhysAddr) -> bool {
    self.as_u64() == rhs.as_u64()
  }
}

#[repr(C)]
#[repr(align(4096))]
pub struct PageList {
  pub pages: [Option<PhysAddr>; PagesPerBlock],
  pub used: [bool; PagesPerBlock],
  pub next: PageListLink,
  pub prev: PageListLink,
  pub lowest: PhysAddr,
  pub highest: PhysAddr,
}

assert_eq_size!(check_page_list_size; PageList, [u8;4096]);

#[repr(C)]
pub struct PageRange {
  pub start: PhysAddr,
  pub pages: usize,
  pub next: PageListLink,
  pub prev: PageListLink,
}

pub enum PageListLink {
  None,
  PageRangeEntry(NonNull<PageRange>), // Start and number of 4096 Pages
  PageListEntry(NonNull<PageList>),
}

impl PageListLink {
  fn is_none(&self) -> bool {
    match self {
      PageListLink::None => true,
      _ => false,
    }
  }
  fn is_range(&self) -> bool {
    match self {
      PageListLink::PageRangeEntry(_) => true,
      _ => false,
    }
  }
  fn is_entry(&self) -> bool {
    match self {
      PageListLink::PageListEntry(_) => true,
      _ => false,
    }
  }
  fn next_any(&self) -> PageListLink {
    if self.is_none() { 
      panic!("next_any went beyond end of list"); 
    };
    match self {
      PageListLink::PageListEntry(pl) => {
        pl.as_ref().next
      }
      PageListLink::PageRangeEntry(pr) => {
        pr.as_ref().next
      }
      PageListLink::None => {
        PageListLink::None
      }
    }
  }
  fn next_range(&self) -> Option<NonNull<PageRange>> {
    let next_any = self.next_any();
    if let PageListLink::PageRangeEntry(pr) = next_any {
      Some(pr)
    } else if let PageListLink::None = next_any {
      None
    } else {
      next_any.next_range()
    }
  }
  fn next_entry(&self) -> Option<NonNull<PageList>> {
    let next_any = self.next_any();
    if let PageListLink::PageListEntry(pl) = next_any {
      Some(pl)
    } else if let PageListLink::None = next_any {
      None
    } else {
      next_any.next_entry()
    }
  }
  fn next_entry_with_free(&self) -> Option<NonNull<PageList>> {
    let next_range = self.next_entry();
    match next_range {
      Some(r) => {
        if r.as_ref().has_free() {
          Some(r)
        } else {
          PageListLink::PageListEntry(r).next_entry_with_free()
        }
      },
      None => None,
    }
  }
  fn get_end(&self) -> PageListLink {
    if self.is_none() { 
      panic!("end of list went beyond end of list"); 
    };
    let next = match self {
      PageListLink::PageListEntry(pl) => {
        pl.as_ref().next
      }
      PageListLink::PageRangeEntry(pr) => {
        pr.as_ref().next
      }
      PageListLink::None => {
        PageListLink::None
      }
    };
    if  next.is_entry() || 
        next.is_range() {
      next.get_end()
    } else {
      *self
    }
  }
  pub fn grab_free(&mut self) -> Option<PhysAddr> {
    match self {
      PageListLink::PageListEntry(pl) => {
        let pldr = pl.as_ref();
        for x in 0..PagesPerBlock {
          if !pldr.used[x] && pldr.pages[x].is_some() {
            pldr.used[x];
            return pldr.pages[x];
          }
        }
      }
      _ => {}
    }
    match self.next_entry_with_free() {
      Some(ne) => PageListLink::PageListEntry(ne).grab_free(),
      None => None,
    }
  }
  pub fn grab_two(&mut self) -> Option<(PhysAddr, PhysAddr)> {
    if let Some(pa) = self.grab_free() {
      if let Some(pa2) = self.grab_free() {
        return Some((pa, pa2))
      } else {
        self.release(pa);
      }
    }
    None
  }
  pub fn release(&mut self, p: PhysAddr) {
    //TODO: zero page content
    //TODO: mark page unused
    panic!("TODO")
  }
  pub fn append_range(&mut self, base: PhysAddr, page_count: usize) {
    //TODO: zero memory range
    //TODO: append range to list
    panic!("TODO")
  }
  pub fn insert_addr(&mut self, addr: PhysAddr) {
    panic!("TODO")
  }
  fn convert_range(&mut self, needed: usize) {
    //TODO: scan for unused list entry slots
    //      then fill from range
    //      if <needed> pages were not filled up
    //      create new, empty range and repeat
    panic!("TODO")
  }
  pub fn free_pages(&self) -> usize {
    //TODO: count number of unused pages
    panic!("TODO")
  }
}

impl PageList {
  pub fn new(p: PhysAddr) -> NonNull<PageList> {
    zero_page(p);
    NonNull::new_unchecked(p.as_u64() as *mut PageList)
  }
  pub fn has_free(&self) -> bool {
    for x in 0..PagesPerBlock {
      if self.used[x] {
        return true;
      }
    }
    return false;
  }
  /*pub fn release(&mut self, p: PhysAddr) {
    if !(p.as_u64() < self.lowest.as_u64() || p.as_u64() > self.highest.as_u64()) {
      for x in 0..PagesPerBlock {
        match self.pages[x] {
          Some(page) => {
            if page == p {
              self.used[x] = false;
              zero_page(p);
              return;
            }
          }
          None => (),
        }
      }
    }
    match self.next {
      PageListLink::PageListEntry(next_pl) => {
        unsafe { (*next_pl.as_ptr()).release(p) }
      },
      PageListLink::PageRangeEntry(pr) => panic!("not implemented"),
      PageListLink::None => panic!("could not release address"),
    }
  }*/
  // convert_range will create a PageList and a PageRange out of a given PageListLink, which must
  // be a PageRangeEntry. The PageRange will have at most "PagesPerBlock" in pages consumed and
  // the adjusted values will be returned.
  /*fn convert_range(&mut self, pll: PageListLink) -> (PageRange, *mut PageList) {
    if let PageListLink::PageRangeEntry(pr) = pll {
      if let Some(pla) = self.grab_free() {
        let pl = PageList::new(pla);
        let pla_e = pla as *mut PageRange;
        // Consume all pages that fit into the block
        (PageListLink::PageRangeEntry(PageRange{
          start: pa.start + PageSize * PagesPerBlock, 
          size: pa.size - PagesPerBlock,
          next: pa.next,
          prev: pa.previous,
        }), pla.as_u64() as *mut PageList)
      } else {
        let new_pa = pa + PageSize * (PagesPerBlock + 1);
        let new_size = size - (PagesPerBlock + 1); // Adjust range
        zero_page(pa);
        (PageListLink::PageRange(new_pa, new_size), pa.as_u64() as *mut PageList)
      }
    } else {
      panic!("attempted to convert non-pagerange to pagelist entry");
    }
  }*/
}

fn zero_page(page: PhysAddr) {
  use ::vmem::PageSize;
  let page_raw = page.as_u64() as *mut [u8; PageSize];
  for x in 0..PageSize {
    unsafe { (*page_raw)[x] = 0x00 };
  }
}
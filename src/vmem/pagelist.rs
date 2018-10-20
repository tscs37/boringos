

const PAGES_PER_BLOCK: usize = 448; // Readjust this when struct layout changes
use ::core::ptr::NonNull;
use ::core::cmp::Ordering;

// we ignore address 0
#[derive(Clone,Copy)]
pub struct PhysAddr(NonNull<u8>);

#[repr(C)]
#[repr(align(4096))]
pub struct PageList {
  pub pages: [Option<PhysAddr>; PAGES_PER_BLOCK],
  pub used: [bool; PAGES_PER_BLOCK],
  //TODO: we don't really need so see the previous block, first is simpler and
  //sufficient here instead of prev
  pub next: PageListLink,
  pub prev: PageListLink,
  pub lowest: PhysAddr,
  pub highest: PhysAddr,
}

#[repr(C)]
#[repr(align(4096))]
#[derive(Debug)]
pub struct PageRange {
  pub start: PhysAddr,
  pub pages: usize,
  pub next: PageListLink,
  pub prev: PageListLink,
}

#[derive(Copy,Clone,Debug,PartialEq)]
pub enum PageListLink {
  None,
  PageRangeEntry(NonNull<PageRange>), // Start and number of 4096 Pages
  PageListEntry(NonNull<PageList>),
}

assert_eq_size!(check_phys_addr_size; PhysAddr,    u64);
assert_eq_size!(check_page_list_size; PageList,    [u8; 4096]);
assert_eq_size!(check_page_range_size; PageRange,  [u8; 4096]);

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

impl ::core::fmt::Debug for PageList {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    f.write_fmt(format_args!("PageList(prev={:#018x},next={:#018x},lowest={},highest={},free={})",
      self.prev.get_ptru64(), self.next.get_ptru64(), 
      self.lowest, self.highest, 
      self.count_free(),
    ))
  }
}

impl ::core::fmt::Display for PageList {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    f.write_fmt(format_args!("{:?}", self))
  }
}

impl ::core::fmt::Display for PageRange {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    f.write_fmt(format_args!("{:?}", self))
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

impl ::core::fmt::Display for PageListLink {
  fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
    match self {
      PageListLink::None => { f.write_str("None") },
      PageListLink::PageListEntry(pe) => { 
        f.write_fmt(format_args!("List={:#018x})", pe.as_ptr() as usize))
      },
      PageListLink::PageRangeEntry(pe) => { 
        f.write_fmt(format_args!("Range={:#018x}", pe.as_ptr() as usize))
      },
    }
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
    PhysAddr(nn)
  }
  pub fn into<T>(self) -> NonNull<T> {
    unsafe { NonNull::new_unchecked(self.0.as_ptr() as *mut T) }
  }
  pub unsafe fn new_unchecked(p: u64) -> PhysAddr {
    PhysAddr(NonNull::new_unchecked(p as *mut u8))
  }
  pub fn new_or_abort(p: u64) -> PhysAddr {
    match PhysAddr::new(p) {
      None => panic!("could not create physaddr for {:#018x}, probably was null or illegal", p),
      Some(pa) => pa
    }
  }
  // Adds the specified number of pages as offset and returns the result as PhysAddr
  pub unsafe fn add_pages(&self, pages: u64) -> PhysAddr {
    PhysAddr(NonNull::new_unchecked(
      (self.as_u64() + (pages * ::vmem::PAGE_SIZE as u64)) as *mut u8))
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


impl PageList {
  pub fn new(p: PhysAddr) -> NonNull<PageList> {
    unsafe { zero_page(p); }
    p.into::<PageList>() 
  }
  pub fn has_free(&self) -> bool {
    for x in 0..PAGES_PER_BLOCK {
      if !self.used[x] && !self.pages[x].is_some() {
        return true;
      }
    }
    return false;
  }
  pub fn has_empty(&self) -> bool {
    for x in 0..PAGES_PER_BLOCK {
      if self.pages[x].is_none() {
        return true;
      }
    }
    return false;
  }
  pub fn count_free(&self) -> usize {
    let mut out = 0;
    for x in 0..PAGES_PER_BLOCK {
      if !self.used[x] && !self.pages[x].is_none() { out += 1; }
    }
    return out;
  }
  pub fn count_empty(&self) -> usize {
    let mut out = 0;
    for x in 0..PAGES_PER_BLOCK {
      if self.pages[x].is_none() {
        out += 1;
      }
    }
    return out;
  }
  // panics if pr is too large
  fn insert_from_range(&mut self, pr: PageRange) {
    if pr.pages > self.count_empty() {
      panic!("attempted to insert range into too small empty PLE");
    }
    trace!("inserting {} pages into list, current={}", pr.pages, self.count_free());
    for x in 0..pr.pages {
      for y in 0..PAGES_PER_BLOCK {
        if self.pages[y].is_none() {
          self.pages[y] = Some(unsafe { pr.start.add_pages(x as u64) });
          break;
        }
      }
    }
    self.update_lowhi();
    trace!("now have {} free pages", self.count_free());
  }
  fn update_lowhi(&mut self) {
    for x in 0..PAGES_PER_BLOCK {
      match self.pages[x] {
        None => (),
        Some(page) => {
          if page.as_u64() < self.lowest.as_u64() {
            self.lowest = page;
          }
          if page.as_u64() > self.highest.as_u64() {
            self.highest = page;
          }
        }
      }
    }
  }
}

impl PageRange {
  pub fn new(p: PhysAddr, base: PhysAddr, size: usize) -> NonNull<PageRange> {
    unsafe {
      let mut pr = PageRange::new_empty(p);
      {
        let prm = pr.as_mut();
        prm.start = base;
        prm.pages = size;
      }
      pr
    }
  }
  pub fn new_empty(p: PhysAddr) -> NonNull<PageRange> {
    unsafe {
      zero_page(p); 
      NonNull::new_unchecked(p.as_u64() as *mut PageRange)
    }
  }
  // splits the range after the given number of pages and returns a new PageRange
  // with no links to a list
  fn sub_pages(&mut self, pages: usize) -> Option<PageRange> {
    if pages > self.pages { None } else {
      let pr = PageRange{
        start: self.start,
        pages: pages,
        next: PageListLink::None,
        prev: PageListLink::None,
      };
      self.start = unsafe { PhysAddr::new_unchecked(
        self.start.as_u64() + (pages * ::vmem::PAGE_SIZE) as u64
      ) };
      self.pages -= pages;
      Some(pr)
    }
  }
}

impl PageListLink {
  #[allow(dead_code)]
  pub fn dump(&self) {
    let mut page = *self;
    loop {
      let page_ptr = page.get_ptru64();
      match page {
        PageListLink::None => { debug!("{:#018x} => None", page_ptr); }
        PageListLink::PageListEntry(ref e) => {
          debug!("{:#018x} => {:?}", page_ptr, unsafe { &*(e.as_ptr()) });
        }
        PageListLink::PageRangeEntry(ref e) => {
          debug!("{:#018x} => {:?}", page_ptr, unsafe { &*(e.as_ptr()) });
        }
      }
      if page.is_none() {
        return;
      }
      page = page.next_any();
    }
  }
  fn get_ptru64(&self) -> u64 {
    match self { 
      PageListLink::None => 0,
      PageListLink::PageListEntry(pl) => pl.as_ptr() as u64,
      PageListLink::PageRangeEntry(pr) => pr.as_ptr() as u64,
    }
  }
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
    match self {
      PageListLink::PageListEntry(pl) => {
        unsafe { pl.as_ref().next }
      }
      PageListLink::PageRangeEntry(pr) => {
        unsafe { pr.as_ref().next }
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
    match self {
      PageListLink::PageListEntry(pl) => {
        if unsafe{pl.as_ref()}.has_free() {
          return Some(*pl)
        }
      }
      _ => {/* ignore non-entry self */}
    }
    let next_range = self.next_entry();
    match next_range {
      Some(r) => {
        if unsafe { r.as_ref() }.has_free() {
          Some(r)
        } else {
          PageListLink::PageListEntry(r).next_entry_with_free()
        }
      },
      None => None,
    }
  }
  fn next_entry_with_empty(&self) -> Option<NonNull<PageList>> {
    match self {
      PageListLink::PageListEntry(pl) => {
        if unsafe{pl.as_ref()}.has_empty() {
          return Some(*pl)
        }
      }
      _ => {/* ignore non-entry self */}
    }
    let next_range = self.next_entry();
    match next_range {
      Some(r) => {
        if unsafe { r.as_ref() }.has_empty() {
          Some(r)
        } else {
          PageListLink::PageListEntry(r).next_entry_with_empty()
        }
      },
      None => None,
    }
  }
  fn get_end(&self) -> PageListLink {
    if self.is_none() { 
      return PageListLink::None;
    }
    let next = match self {
      PageListLink::PageListEntry(pl) => {
        unsafe { pl.as_ref().next }
      }
      PageListLink::PageRangeEntry(pr) => {
        unsafe { pr.as_ref().next }
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
  fn get_start(&self) -> PageListLink {
    if self.is_none() { 
      return PageListLink::None;
    }
    let prev = match self {
      PageListLink::PageListEntry(pl) => {
        unsafe { pl.as_ref().prev }
      }
      PageListLink::PageRangeEntry(pr) => {
        unsafe { pr.as_ref().prev }
      }
      PageListLink::None => {
        PageListLink::None
      }
    };
    if  prev.is_entry() || 
        prev.is_range() {
      prev.get_start()
    } else {
      *self
    }
  }
  pub fn set_next(&mut self, p: PageListLink) {
    match self {
      PageListLink::PageListEntry(pl) => { unsafe{pl.as_mut()}.next = p },
      PageListLink::PageRangeEntry(pr) => { unsafe{pr.as_mut()}.next = p },
      PageListLink::None => { panic!("attempted to set next of none"); }
    }
  }
  pub fn set_prev(&mut self, p: PageListLink) {
    match self {
      PageListLink::PageListEntry(pl) => { unsafe{pl.as_mut()}.prev = p },
      PageListLink::PageRangeEntry(pr) => { unsafe{pr.as_mut()}.prev = p },
      PageListLink::None => { panic!("attempted to set prev of none"); }
    }
  }
  pub fn get_prev(&self) -> PageListLink {
    match self {
      PageListLink::None => PageListLink::None,
      PageListLink::PageListEntry(ple) => unsafe{ple.as_ref()}.prev,
      PageListLink::PageRangeEntry(pre) => unsafe{pre.as_ref()}.prev,
    }
  }
  pub fn grab_free(&mut self) -> Option<PhysAddr> {
    trace!("grabbing a free page from memory");
    match self {
      PageListLink::PageListEntry(pl) => {
        let pldr = unsafe { pl.as_mut() };
        for x in 0..PAGES_PER_BLOCK {
          if !pldr.used[x] && pldr.pages[x].is_some() {
            pldr.used[x] = true;
            let addr = pldr.pages[x].
              expect("nonused page grabbed but was none");
            return Some(addr);
          }
        }
      }
      _ => {
      }
    }
    // skip one entry ahead to avoid recursion into self
    match self.next_any().next_entry_with_free() {
      Some(ne) => {
        PageListLink::PageListEntry(ne).grab_free()
      },
      None => {
        match self.get_start().convert_range(1) {
          Ok(pages) => trace!("converted {} pages", pages),
          Err(e) => panic!("conversion failed: {}", e),
        }
        match self.get_start().next_entry_with_free() {
          Some(ne) => {
            PageListLink::PageListEntry(ne).grab_free()
          },
          None => {
            None
          },
        }
      },
    }
  }
  pub fn release(&mut self, p: PhysAddr) {
    unsafe { zero_page(p) };
    let mut cur = Some(*self);
    loop {
      match cur {
        None => panic!("release page encountered empty entry in pagelist"),
        Some(cur_o) => {
          match cur_o {
            PageListLink::None => {
              warn!("page {} not tracked, leaking it", p);
              return;
            },
            PageListLink::PageListEntry(mut ple_ptr) => {
              trace!("checking page list: {}", cur_o);
              let ple = unsafe { ple_ptr.as_mut() };
              if !(ple.highest < p || ple.lowest > p) {
                for x in 0..PAGES_PER_BLOCK {
                  if let Some(ple_pa) = ple.pages[x] {
                    trace!("check if page matches {}", ple_pa);
                    if ple_pa == p {
                      trace!("found page, marking unused");
                      ple.used[x] = false;
                      return;
                    }
                  }
                }
              }
              cur = Some(cur_o.next_any());
            }
            PageListLink::PageRangeEntry(_) => {
              cur = Some(cur_o.next_any());
            }
          }
        }
      }
    }
  }
  pub fn append_range(&mut self, base: PhysAddr, page_count: usize) {
    let mut end = self.get_end();
    let page = self.grab_free();
    match page {
      None => panic!("could not allocate for page range append"),
      Some(p) => {
        let mut range = PageRange::new(p, base, page_count);
        unsafe{
          let rref = range.as_mut();
          rref.next = PageListLink::None;
          rref.prev = end;
        }
        end.set_next(PageListLink::PageRangeEntry(range));
      }
    }
  }
  fn convert_range<'a>(&mut self, needed: usize) -> Result<usize, &'a str> {
    let next_range = self.get_start().next_range();
    if next_range.is_none() {
      return Err("there is no range installed anywhere");
    }
    let mut copy_self = *self;
    match copy_self {
      PageListLink::None => Err("attempted to convert after end of list"),
      PageListLink::PageRangeEntry(_) => Err("cannot convert page entry"),
      PageListLink::PageListEntry(ref mut p) => {
        let pref: &mut PageList = unsafe{p.as_mut()};
        if pref.has_empty() {
          if pref.count_empty() >= needed {
            match next_range {
              None => Err("could not get any range to convert to memory"),
              Some(mut range) => {
                let rref: &mut PageRange = unsafe{range.as_mut()};
                if rref.pages >= needed {
                  match rref.sub_pages(needed) {
                    None => Err("rref refused to subdivide"),
                    Some(er) => {
                      trace!("inserting range {:?} into pref", er);
                      pref.insert_from_range(er);
                      if rref.pages == 0 {
                        trace!("page range has no pages left, cleaning it up");
                        let range_entry = PageListLink::PageRangeEntry(
                          unsafe{NonNull::new_unchecked(rref)});
                        let mut prev = range_entry.get_prev();
                        let mut next = range_entry.next_any();
                        assert_ne!(prev, PageListLink::None, "list can only cut toward end, cannot cut at first page");
                        if next == PageListLink::None {
                          prev.set_next(next);
                        } else {
                          prev.set_next(next);
                          next.set_prev(prev);
                        }
                        self.release(PhysAddr::new_or_abort(rref as *mut _ as u64));
                      }
                      return Ok(needed);
                    }
                  }
                } else {
                  let max_needed = needed- rref.pages;
                  // convert all pages we have
                  return match self.convert_range(rref.pages) {
                    Ok(a) => {
                      match self.convert_range(max_needed) {
                        Ok(b) => Ok(a+b),
                        Err(e) => Err(e),
                      }
                    }
                    Err(e) => Err(e),
                  };
                }
              }
            }
          } else {
            let max_fit = pref.count_empty();
            let new_needed = needed - max_fit;
            let mut e = PageListLink::PageListEntry(NonNull::from(pref));
            let captured = match e.convert_range(max_fit) {
              Ok(n) => n,
              Err(s) => { return Err(s) }
            };
            match e.convert_range(new_needed) {
              Ok(m) => Ok(captured+m),
              Err(s) => Err(s),
            }
          }
        } else {
          let mut e = PageListLink::PageListEntry(NonNull::from(pref));
          match e.get_start().next_entry_with_empty() {
            Some(e) => PageListLink::PageListEntry(e).
              convert_range(needed),
              //TODO: allocate new list
              None => Err("no further empty slots")
          }
        }
      }
    }
  }
  pub fn free_pages(&self) -> usize {
    let self_count = match self {
      PageListLink::None => 0,
      PageListLink::PageRangeEntry(p) => unsafe{p.as_ref()}.pages,
      PageListLink::PageListEntry(p) => unsafe{p.as_ref()}.count_free(),
    };
    let next = self.next_any();
    match next {
      PageListLink::None => self_count,
      _ => self_count + next.free_pages(),
    }
  }
}

unsafe fn zero_page(page: PhysAddr) {
  use ::vmem::PAGE_SIZE;
  let page_raw = page.as_u64() as *mut [u8; PAGE_SIZE];
  for x in 0..PAGE_SIZE {
    (*page_raw)[x] = 0x00;
  }
}
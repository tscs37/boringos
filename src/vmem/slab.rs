
use spin::Mutex;
use core::alloc::Layout;
use core::ptr::NonNull;

#[derive(Debug)]
pub struct Slab {
  address: *mut u8,
  size: usize,
}

impl Slab {
  pub fn address(&self) -> *mut u8 {
    self.address
  }
  pub fn size(&self) -> usize {
    self.size
  }
  pub fn from_ptr(ptr: NonNull<u8>, layout: Layout) -> Slab {
    Slab{
      address: ptr.as_ptr(),
      size: layout.size()
    }
  }
}

use alloc::collections::{BinaryHeap, BTreeSet};
use core::cell::RefCell;

pub struct SlabFrameAllocator {
  size: usize,
  freemem: Rc<UnsafeCell<BinaryHeap<*mut u8>>>,
  usedmem: Rc<UnsafeCell<BTreeSet<*mut u8>>>,
}

impl SizedAllocator for SlabFrameAllocator {
  fn allocate_slab(&mut self) -> Option<Slab> {
    //debug!("Allocating slab with {} byte", self.slab_size());
    let freemem = self.get_freemem();
    let usedmem = self.get_usedmem();
    if let Some(memaddr) = freemem.pop() {
      debug!("Allocating slab with {} byte at {:#016x}", self.slab_size(), memaddr as usize);
      usedmem.insert(memaddr);
      debug!("Marked memory as used");
      return Some(Slab{
        address: memaddr, 
        size: self.slab_size() 
        });
    } else {
      debug!("allocation failed due to empty free memory, you may have to steal a slab");
      return None;
    }
  }
  fn deallocate_slab(&mut self, slab: Slab) {
    debug!("Deallocating slab with {} byte at {:#016x}", self.slab_size(), slab.address as usize);
    let freemem = self.get_freemem();
    let usedmem = self.get_usedmem();
    if slab.size != self.slab_size() { 
      panic!("dealloc of foreign frame {} != {}", slab.size, self.slab_size()); 
    }
    if usedmem.remove(&slab.address) {
      freemem.push(slab.address);
    } else {
      panic!("Frame to deallocate not found: {:?}", slab);
    }
  }
  fn slab_size(&self) -> usize { self.size }

  unsafe fn push_slab(&mut self, addr: usize) {
    debug!("Adding slab at {:#016x} (size: {})", addr, self.slab_size());
    self.get_freemem().push(addr as *mut u8);
  }
  fn has_free(&self) -> bool {
    debug!("free slabs: {}", self.get_freemem().len());
    self.get_freemem().len() > 0
  }
  fn count_free(&self) -> usize {
    self.get_freemem().len()
  }
}

impl SlabFrameAllocator {
  pub fn new(size: usize) -> SlabFrameAllocator {
    SlabFrameAllocator{
      size: size,
      freemem: Rc::new(UnsafeCell::new(BinaryHeap::new())),
      usedmem: Rc::new(UnsafeCell::new(BTreeSet::new())),
    }
  }

  pub fn new_cap(size: usize, initial_cap: usize) -> SlabFrameAllocator {
    let true_cap = (initial_cap / size) - 1;
    SlabFrameAllocator{
      size: size,
      freemem: Rc::new(UnsafeCell::new(BinaryHeap::with_capacity(true_cap))),
      usedmem: Rc::new(UnsafeCell::new(BTreeSet::new())),
    }
  }

  fn get_freemem<'a>(&self) -> &'a mut BinaryHeap<*mut u8> {
    if (*self.freemem).get().is_null() { panic!("freemem bheap was null pointer"); }
    unsafe { &mut *(*self.freemem).get() }
  }
  fn get_usedmem<'a>(&self) -> &'a mut BTreeSet<*mut u8> {
    if (*self.usedmem).get().is_null() { panic!("usedmem btree was null pointer"); }
    unsafe { &mut *(*self.usedmem).get() }
  }
}

pub trait SizedAllocator {
  fn allocate_slab(&mut self) -> Option<Slab>;
  fn deallocate_slab(&mut self, slab: Slab);
  fn has_free(&self) -> bool;
  fn count_free(&self) -> usize;
  fn slab_size(&self) -> usize;
  unsafe fn push_slab(&mut self, addr: usize);
}

use alloc::rc::Rc;
use core::cell::UnsafeCell;

pub struct SlabCollection {
  // kernel alloc
  slab_64:    Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_128:   Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_256:   Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_1k:    Rc<UnsafeCell<SlabFrameAllocator>>,
  // public alloc
  slab_4k:    Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_16k:   Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_64k:   Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_1m:    Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_16m:   Rc<UnsafeCell<SlabFrameAllocator>>,
  slab_128m:  Rc<UnsafeCell<SlabFrameAllocator>>,
}

pub fn conv_spr<'a>(sfa_ptr: &Rc<UnsafeCell<SlabFrameAllocator>>) -> &'a mut SlabFrameAllocator {
  if (*sfa_ptr.clone()).get().is_null() { panic!("SFA was null pointer"); }
  unsafe { &mut *(*sfa_ptr.clone()).get() }
}

impl SlabCollection {
  pub fn new() -> SlabCollection {
    SlabCollection{
      slab_64: Rc::new(UnsafeCell::new(SlabFrameAllocator::new_cap(64, 4096))),
      slab_128: Rc::new(UnsafeCell::new(SlabFrameAllocator::new_cap(128, 4096))),
      slab_256: Rc::new(UnsafeCell::new(SlabFrameAllocator::new_cap(256, 4096))),
      slab_1k: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(1024))),
      slab_4k: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(4 * 1024))),
      slab_16k: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(16 * 1024))),
      slab_64k: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(64 * 1024))),
      slab_1m: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(1 * 1024 * 1024))),
      slab_16m: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(16 * 1024 * 1024))),
      slab_128m: Rc::new(UnsafeCell::new(SlabFrameAllocator::new(128 * 1024 * 1024))),
    }
  }

  pub fn push_slab(&self, addr: usize, size: usize) -> Result<usize, &'static str> {
    match self.select_sfa_slab(size) {
      Some(sfa_ptr) => {
        use vmem::slab::SizedAllocator;
        let mut sfa = conv_spr(&sfa_ptr);
        //debug!("Found Slab Allocator for size {} => {}", size, sfa.slab_size());
        let base = addr;
        let slab_size = sfa.slab_size();
        let num_slabs = size / slab_size;
        debug!("Adding {} slabs to memory", num_slabs);
        for x in 0..num_slabs {
          unsafe { sfa.push_slab(addr + x * slab_size) };
        }
        return Ok(sfa.slab_size())
      }
      None => {
        debug!("For size {}, no slab allocator found", size);
        return Err("no slab alloc found for size {}")
      }
    }
  }

  pub fn has_free(&self) -> bool {
    //debug!("checking if we have free slabs");
    SlabCollection::check_slab_free(&self.slab_64) ||
    SlabCollection::check_slab_free(&self.slab_128) ||
    SlabCollection::check_slab_free(&self.slab_256) ||
    SlabCollection::check_slab_free(&self.slab_1k) ||
    SlabCollection::check_slab_free(&self.slab_4k) ||
    SlabCollection::check_slab_free(&self.slab_16k) ||
    SlabCollection::check_slab_free(&self.slab_64k) ||
    SlabCollection::check_slab_free(&self.slab_1m) ||
    SlabCollection::check_slab_free(&self.slab_16m) ||
    SlabCollection::check_slab_free(&self.slab_128m)
  }

  pub fn count_free(&self) -> usize {
    //debug!("counting free memory");
    SlabCollection::count_slab_free(&self.slab_64) +
    SlabCollection::count_slab_free(&self.slab_128) +
    SlabCollection::count_slab_free(&self.slab_256) +
    SlabCollection::count_slab_free(&self.slab_1k) +
    SlabCollection::count_slab_free(&self.slab_4k) +
    SlabCollection::count_slab_free(&self.slab_16k) +
    SlabCollection::count_slab_free(&self.slab_64k) +
    SlabCollection::count_slab_free(&self.slab_1m) +
    SlabCollection::count_slab_free(&self.slab_16m) +
    SlabCollection::count_slab_free(&self.slab_128m)
  }

  pub fn check_slab_free(s: &Rc<UnsafeCell<SlabFrameAllocator>>) -> bool {
    conv_spr(&s).has_free()
  }

  pub fn count_slab_free(s: &Rc<UnsafeCell<SlabFrameAllocator>>) -> usize {
    let sq = unsafe { &mut *(*s).get() };
    let counter = sq.count_free();
    //debug!("slab-size {} has {} slabs free", sq.slab_size(), counter);
    counter * sq.slab_size()
  }

  fn select_sfa(&self, size: usize) -> Option<Rc<UnsafeCell<SlabFrameAllocator>>> {
    //debug!("loading slab allocator for size {}", size);
    let mut selected_sfa: Option<Rc<UnsafeCell<SlabFrameAllocator>>> = None;
    if size <= 64 { selected_sfa = Some(self.slab_64.clone()) }
    else if size <= 128 { selected_sfa = Some(self.slab_128.clone()) }
    else if size <= 256 { selected_sfa = Some(self.slab_256.clone()) }
    else if size <= 1024 { selected_sfa = Some(self.slab_1k.clone()) }
    else if size <= 4 * 1024 { selected_sfa = Some(self.slab_4k.clone()) }
    else if size <= 16 * 1024 { selected_sfa = Some(self.slab_16k.clone()) }
    else if size <= 64 * 1024 { selected_sfa = Some(self.slab_64k.clone()) }
    else if size <= 1 * 1024 * 1024 { selected_sfa = Some(self.slab_1m.clone()) }
    else if size <= 16 * 1024 * 1024 { selected_sfa = Some(self.slab_16m.clone()) }
    else if size <= 128 * 1024 * 1024 { selected_sfa = Some(self.slab_128m.clone()) }
    else { selected_sfa = None /* Return None => Allocation too Large, split up */ }

    match selected_sfa {
      None => return selected_sfa,
      Some(sfa) => {
        if conv_spr(&sfa).has_free() {
          //debug!("slab allocator is free, using it");
          return Some(sfa.clone());
        } else {
          debug!("need to steal a slab for {}, loading next slab alloc", 
            conv_spr(&sfa).slab_size());
          let next_sfa = self.select_sfa( conv_spr(&sfa).slab_size() + 1);
          match next_sfa {
            Some(next_sfa_unwrapped) => {
              debug!("next slab alloc has {} sized slab",  conv_spr(&next_sfa_unwrapped).slab_size());
              match unsafe { SlabCollection::steal_slab(next_sfa_unwrapped.clone(), sfa.clone()) } {
                Ok(num) => {
                  debug!("stole for {} slabs", num);
                  return Some(sfa.clone());
                }
                Err(msg) => {
                  debug!("error trying to steal slabs: {}", msg);
                  return None;
                }
              }
            }
            None => {
              debug!("could not steal slab, no next sfa found");
              return None;
            }
          }
        }
      }
    }
  }

  // returns Ok() with the number of new slabs added to the allocator
  unsafe fn steal_slab<'a>(
    sa_from: Rc<UnsafeCell<SlabFrameAllocator>>, sa_to: Rc<UnsafeCell<SlabFrameAllocator>>) -> Result<usize, &'a str> {
      //debug!("stealing slab from outer slab allocator");
      if let Some(slab) = conv_spr(&sa_from).allocate_slab() {
        debug!("moving slab into inner slab allocator");
        let sa_inner = conv_spr(&sa_to);
        if slab.size() < sa_inner.slab_size() {
          panic!("attempted to steal slab into bigger allocator");
        }
        let num_slabs = (slab.size() / sa_inner.slab_size());
        for x in 0..num_slabs {
          let ptr = (slab.address as usize) + x * sa_inner.slab_size();
          sa_inner.push_slab(ptr);
        }
        return Ok(num_slabs);
      } else {
        return Err("could not allocate slab, got nothing in return");
      }
  }

  unsafe fn steal_slab_from_boot<'a>(
    bm: &Mutex<super::bmfa::BitmapFrameAllocator>,
    sa_to: Rc<UnsafeCell<SlabFrameAllocator>>) -> Result<usize, &'a str> {
      debug!("Stealing slab memory from boot memory");
      let sa = conv_spr(&sa_to);
      if sa.slab_size() > 4096 {
        panic!("boot_memory can only steal slabs of 4096 byte or less");
      }
      use vmem::fmem::FrameAllocator;
      if let Some(frame) = bm.lock().allocate_frame() {
        let num_slabs = (4096 / sa.slab_size())-1;
        debug!("pushing {} slabs from boot memory", num_slabs);
        use vmem::fmem::PageSize;
        let frame_base = frame.number * PageSize;
        for x in 0..num_slabs {
          let ptr = frame_base + x * sa.slab_size();
          sa.push_slab(ptr);
        }
        debug!("{} slabs added to slab memory");
        Ok(num_slabs)
      } else {
        panic!("could not alloc from boot memory")
      }
  }

  pub unsafe fn init_steal_first(&self) {
    // init low memory
    SlabCollection::steal_slab(self.slab_128m.clone(), self.slab_16m.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_16m.clone(), self.slab_1m.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_1m.clone(), self.slab_64k.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_64k.clone(), self.slab_16k.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_16k.clone(), self.slab_4k.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_16k.clone(), self.slab_1k.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_1k.clone(), self.slab_256.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_1k.clone(), self.slab_128.clone()).unwrap();
    SlabCollection::steal_slab(self.slab_1k.clone(), self.slab_64.clone()).unwrap();
  }

  pub unsafe fn init_steal_boot(&self, 
    bm: &Mutex<super::bmfa::BitmapFrameAllocator>) {
    SlabCollection::steal_slab_from_boot(&bm, self.slab_64.clone()).
      expect("steal_boot at 64 must unwrap");
    SlabCollection::steal_slab_from_boot(&bm, self.slab_128.clone()).
      expect("steal_boot at 128 must unwrap");
    SlabCollection::steal_slab_from_boot(&bm, self.slab_256.clone()).
      expect("steal_boot at 256 must unwrap");
  }

  pub fn select_sa(&self, size: usize) -> Option<Rc<UnsafeCell<SlabFrameAllocator>>> {
    return self.select_sfa(size);
  }

  fn select_sfa_slab(&self, size: usize) -> Option<Rc<UnsafeCell<SlabFrameAllocator>>> {
    if size <= 4 * 1024 { 
      debug!("slab too small: {}", size);
      return None /* No slab smaller than 4k for new slabs */ 
    }
    else if size <= 16 * 1024 { return Some(self.slab_4k.clone()) }
    else if size <= 64 * 1024 { return Some(self.slab_16k.clone()) }
    else if size <= 1 * 1024 * 1024 { return Some(self.slab_64k.clone()) }
    else if size <= 16 * 1024 * 1024 { return Some(self.slab_1m.clone()) }
    else if size <= 128 * 1024 * 1024 { return Some(self.slab_16m.clone()) }
    else { return Some(self.slab_128m.clone()) }
  }
}
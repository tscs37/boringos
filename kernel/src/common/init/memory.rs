
use crate::common::*;
use crate::common::ALLOCATOR;
use crate::vmem;

fn init_physical_memory_offset(boot_info: &'static bootloader::BootInfo) {
  debug!("creating kernel vmem");
  unsafe{
    crate::vmem::pagetable::init(VirtAddr::new(boot_info.physical_memory_offset));
    pager()
      .init(VirtAddr::new(boot_info.physical_memory_offset))
      .expect("init on pager failed");
  };
  debug!("kernel vmem initialized")
}

fn init_allocator() {
  let start = vmem::KHEAP_ALLOC;
  let size = vmem::KHEAP_END - vmem:: KHEAP_START;
  debug!("initializing allocator from {:#018x} with {} pages, {} MiB", start, size / 4096, size / 4096 / 1024 / 1024);
  let vaddr = VirtAddr::new(start.try_into().unwrap());
  vmem::mapper::map_new(vaddr, vmem::mapper::MapType::Data);
  let flags = vmem::mapper::get_flags(vaddr)
    .expect("unmapped allocator page start");
  use x86_64::structures::paging::PageTableFlags;
  assert!(flags.contains(PageTableFlags::WRITABLE));
  unsafe { ALLOCATOR.lock().init(start, size) };
  debug!("kernel allocator initialized");
}

use bootloader::bootinfo::MemoryRegion;
const MAX_MEMORY_MAP_SIZE: usize = 64;

type OptionalRegions<'a> = [Option<&'a MemoryRegion>; MAX_MEMORY_MAP_SIZE];

fn collect_usable_memory(boot_info: &'static bootloader::BootInfo) -> OptionalRegions<'static> {
  let mmap = &boot_info.memory_map;
  let mut usable_entries: OptionalRegions = [None; MAX_MEMORY_MAP_SIZE];
  for (idx, entry) in mmap.iter().enumerate() {
    use bootloader::bootinfo::MemoryRegionType::*;
    match entry.region_type {
      Usable => {
        let range = entry.range;
        let size = (range.end_addr() - range.start_addr()) / 4096;
        debug!(
          "MMAPE {:#04x} is usable memory... {} KiBytes, {} Pages",
          idx,
          size * 4,
          size
        );
        usable_entries[idx] = Some(entry)
      },
      _ => (),
    }
  }
  return usable_entries;
}

pub fn init_memory(boot_info: &'static bootloader::BootInfo) {
  debug!("Probing existing memory ...");
  {
    init_physical_memory_offset(boot_info);
    init_allocator();
    for (idx, entry) in collect_usable_memory(boot_info).iter().enumerate() {
      match entry {
        None => (),
        Some(entry) => {
          let range = entry.range;
          let size = (range.end_addr() - range.start_addr()) / 4096;
          use crate::vmem::pagelist::pagelist_ng::PageMap;
          use alloc::alloc::{Global};
          let layout = PageManager::pagemap_layout();
          let mut rem_pages: u64 = size;
          let mut total_added_pages: u64 = 0;
          while rem_pages > 0 { 
            use core::alloc::GlobalAlloc;
            let ptr = unsafe{ALLOCATOR.alloc_zeroed(layout)};
            let ptr: *mut PageMap = ptr as *mut PageMap;
            let added_pages = unsafe { match pager().add_memory(
              ptr,
              PhysAddr::new(range.start_addr() + (total_added_pages as u64 * 4096)),
                rem_pages.try_into().unwrap()) 
              {
                Ok(v) => v,
                Err(pae) => panic!("could not add memory: {:?}", pae),
              }
            };
            rem_pages -= added_pages;
            total_added_pages += added_pages;
            assert!((rem_pages as u64) < size, "rem_pages has overflown");
          }
        }
      }
    }
  }
}
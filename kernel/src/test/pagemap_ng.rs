
#[test_case]
fn test_pagemap_noalloc() {
  use x86_64::PhysAddr;
  let stackpm: [u8; 4096] = [0; 4096];
  let stackpma = PhysAddr::new(&stackpm as *const u8 as u64);

  use crate::vmem::pagelist::pagelist_ng::PageMap;
  let pagemap = PageMap::new_no_alloc(stackpma, 16);
}
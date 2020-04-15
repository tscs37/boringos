
use linked_list_allocator::Heap;
use spin::Mutex;
use alloc::alloc::{GlobalAlloc, AllocErr, Layout};
use core::ops::Deref;
use core::ptr::NonNull;

use linked_list_allocator::LockedHeap;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();
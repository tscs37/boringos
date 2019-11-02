use super::StateError;
use core::marker::Sized;
use alloc::boxed::Box;

pub trait StateLoader<'a> {
  fn init(data: &'a [u8]) -> Result<Self, StateError> where Self: Sized;
  fn text(&self) -> Box<[u8]>;
  fn data(&self) -> Box<[u8]>;
  fn entry(&self) -> u64;
}
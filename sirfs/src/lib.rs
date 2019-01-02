#![no_std]
#![feature(alloc)]
#![feature(try_from)]

#[macro_use]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::prelude::ToString;
use core::convert::{TryFrom, TryInto};
#[macro_use]
extern crate bitflags;

const MAGIC: &'static[u8] = b"\x00SIRFS\xFF\xFF";

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    D(u16, u8), // High and Low Position
    F(u16, u8), // High and Low Position
    None,
}

impl Entry {
    fn len(self) -> u32 {
        match self {
            Entry::None => 0,
            Entry::F(h, l) => (h as u32) << 8 & (l as u32),
            Entry::D(h, l) => (h as u32) << 8 & (l as u32),
        }
    }
}
impl Into<u32> for Entry {
    fn into(self) -> u32 {
        match self {
            Entry::None =>    0x0 << 24 & (0 as u32) << 8 & (0 as u32),
            Entry::D(h, l) => 0x1 << 24 & (h as u32) << 8 & (l as u32),
            Entry::F(h, l) => 0x2 << 24 & (h as u32) << 8 & (l as u32),
        }
    }
}

impl TryFrom<u32> for Entry {
    type Error = String;

    fn try_from(d: u32) -> Result<Entry, Self::Error> {
        let mode =     (d & 0xFF000000 >> 24) as u8;
        let high_off = (d & 0x00FFFF00 >> 8) as u16;
        let low_off =  (d & 0x000000FF) as u8;
        match mode {
            0 => Ok(Entry::None),
            1 => Ok(Entry::D(high_off, low_off)),
            2 => Ok(Entry::F(high_off, low_off)),
            _ => Err("invalid entry mode".to_string())
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entries([(FileName, Entry); 16]);

impl Into<Vec<u8>> for Entries {
    fn into(self) -> Vec<u8> {
        let mut f: Vec<u8> = Vec::new();
        for x in 0..self.0.len() {
            let ent_f = self.0[x];
            let filename = ent_f.0;
            let filename: [u8; 32] = filename.into();
            let ent: u32 = ent_f.1.into();
            let mut data: Vec<u8> = Vec::new();
            data.extend_from_slice(&filename);
            data.extend_from_slice(&u32_into_vec(ent.into()));
            f.extend_from_slice(&data);
        }
        f
    }
}

impl From<[(FileName, Entry); 16]> for Entries {
    fn from(d: [(FileName, Entry); 16]) -> Entries {
        Entries(d)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileName([u8; 32]);

impl TryFrom<String> for FileName {
    type Error = String;

    fn try_from(s: String) -> Result<FileName, Self::Error> {
        if s.len() > 32 { return Err("filename must be shorter than 32 bytes".to_string())}
        let b = s.bytes();
        let mut f: Vec<u8> = Vec::new();
        for x in b {
            f.push(x)
        }
        let mut q = [0 as u8; 32];
        for x in 0..f.len() {
            q[x] = f[x]
        }
        Ok(FileName(q))
    }
}

impl TryInto<String> for FileName {
    type Error = alloc::string::FromUtf8Error;

    fn try_into(self) -> Result<String, Self::Error> {
        String::from_utf8(self.0.to_vec())
    }
}

impl Into<Vec<u8>> for FileName {
    fn into(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl Into<[u8; 32]> for FileName {
    fn into(self) -> [u8; 32] {
        self.0.clone()
    }
}

bitflags! {
    struct FileFlags: u8 {
        const Executable        = 0b0000_0001;
        const CompressedZLIB    = 0b0000_0010;
    }
}

pub struct FileSystem {
    data: Vec<u8>,
}

#[repr(C)]
#[derive(Debug,PartialEq)]
pub struct Superblock {
    magic: [u8; 8],
    creation_time: u64,
    root_dir_offset: u16,
}

fn vec_into_u64(v: &mut Vec<u8>) -> u64 {
    u64::from_be_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7],])
}

fn u64_into_vec(i: u64) -> Vec<u8> {
    i.to_be_bytes().to_vec()
}

fn vec_into_u32(v: &mut Vec<u8>) -> u32 {
    u32::from_be_bytes([v[0], v[1], v[2], v[3],])
}

fn u32_into_vec(i: u32) -> Vec<u8> {
    i.to_be_bytes().to_vec()
}

fn vec_into_u16(v: &mut Vec<u8>) -> u16 {
    u16::from_be_bytes([v[0], v[1],])
}

fn u16_into_vec(i: u16) -> Vec<u8> {
    i.to_be_bytes().to_vec()
}

impl Into<Vec<u8>> for Superblock {
    fn into(self) -> Vec<u8> {
        let mut d: Vec<u8> = vec!();
        d.extend_from_slice(&self.magic);
        d.extend_from_slice(&u64_into_vec(self.creation_time));
        d
    }
}

#[repr(C)]
#[derive(Debug,PartialEq)]
pub struct Directory {
    files: Entries,
}

impl Directory {
    fn new() -> Directory {
        Directory{
            files: [("".to_string().try_into().unwrap(), Entry::None); 16].into(),
        }
    }
    fn len() -> usize {
        let d: Vec<u8> = Directory::new().into();
        d.len()
    }
}

impl Into<Vec<u8>> for Directory {
    fn into(self) -> Vec<u8> {
        self.files.into()
    }
}

#[repr(C)]
#[derive(Debug,PartialEq)]
pub struct File {
    content_size: u32,
    content_rel_offset: u32,
    flags: FileFlags,
}

impl FileSystem {
    fn open<'a, B>(data: B) -> Self where B: Into<&'a [u8]> { panic!(); }
    fn read<'a, S>(&self, path: S) -> &'a[u8] where S: Into<String> { panic!(); }
    fn stat<'a, S, B>(&self, path: S) -> Result<&'a File, S> where S: Into<String>, B: Into<String> { panic!(); }
    fn chk<S, B>(&self, path: S) -> Result<(), B> where S: Into<String>, B: Into<String> { panic!(); }
}

impl FileSystem {
    // Create a new Filesystem
    fn new() -> Self {
        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&Vec::from(MAGIC.clone()));
        let mut dir_data: Vec<u8> = Directory::new().into();
        data.extend_from_slice(&dir_data);
        FileSystem{data: data,}
    }
    // Add a file in the given directory and with a given filename an data
    fn add_file<'a, S, B, C, D>(&mut self, dir: S, name: B, data: C) -> Result<(), D> where S: Into<String>, B: Into<String>, C: Into<&'a[u8]>, D: Into<String> { panic!(); }
    // Create a directory
    fn create_directory<S, B, C>(&mut self, dir: S, name: B) -> Result<(), C> where S:  Into<String>, B: Into<String>, C:  Into<String> { panic!(); }
    // Close the creation and output a binary block of the data
    fn finish<'a, S>(self) -> S where S: Into<&'a[u8]> { panic!(); }
}

#[cfg(test)]
mod tests;
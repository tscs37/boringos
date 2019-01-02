use crate::*;

#[test]
fn new_fs() {
  assert_eq!(MAGIC.len(), 8, "Magic must have 8 bytes");
  let s = FileSystem::new();
  assert_eq!(s.data.len(), MAGIC.len() + Directory::len(), "Superblock must have correct size");
  assert_eq!(s.data[0..MAGIC.len()], *MAGIC, "Superblock must start mit magic");
  let d: Vec<u8> = Directory::new().into();
  assert_eq!(s.data[MAGIC.len()+1..(s.data.len() - MAGIC.len())], *(d.as_slice()), "Root Directory must be empty and created at 0");
}

#[test]
fn test_u_conv() {
  {
    let num = 0xFF_13_71_00__2A_B0_F0_FF;
    let vec = vec!(0xFF, 0x13, 0x71, 0x00, 0x2A, 0xB0, 0xF0, 0xFF);
    let v = u64_into_vec(num);
    assert_eq!(v, vec, "Convert u64 properly, little endian");
    let q = vec_into_u64(&mut vec.clone());
    assert_eq!(q, num, "Convert vec propertly, little endian");
  }

  {
    let num = 0xFF_13_71_00;
    let vec = vec!(0xFF, 0x13, 0x71, 0x00);
    let v = u32_into_vec(num);
    assert_eq!(v, vec, "Convert u64 properly, little endian");
    let q = vec_into_u32(&mut vec.clone());
    assert_eq!(q, num, "Convert vec propertly, little endian");
  }

  {
    let num = 0xFF_13;
    let vec = vec!(0xFF, 0x13);
    let v = u16_into_vec(num);
    assert_eq!(v, vec, "Convert u64 properly, little endian");
    let q = vec_into_u16(&mut vec.clone());
    assert_eq!(q, num, "Convert vec propertly, little endian");
  }
}
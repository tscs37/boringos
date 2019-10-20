
pub fn get_u128() -> u128 {
  let mut slice: [u8; 16] = [0; 16];
  for x in 0..16 {
    slice[x] = (get_u64() & 0xFF) as u8;
  }
  u128::from_le_bytes(slice)
}

pub fn get_u64() -> u64 {
  if !crate::bindriver::cpu::has_rdrand() {
    panic!("RDRAND but kernel compiled without internal RNG")
  } else {
    let rnd: u64;
    let retry: u32;
    unsafe{
      asm!(
        "
        mov ecx, 1000
        retry_handle_gen:
          rdrand rax
          jc .done_handle_gen
          loop retry_handle_gen
        .done_handle_gen:
        ":
        "={rax}"(rnd), "={ecx}"(retry)::"rax", "ecx":"intel", "volatile"
      );
    }
    if retry == 0 { panic!("could not get random number")}
    rnd
  }
}
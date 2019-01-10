use rand_chacha::ChaChaRng;
use rand::RngCore;
use rand::SeedableRng;
use spin::Mutex;

lazy_static! {
  //TODO: per CPU
  static ref rng: Mutex<ChaChaRng> = Mutex::new(ChaChaRng::seed_from_u64(134_304_831));
}

pub fn get_u128() -> u128 {
  let mut slice: [u8; 16] = [0; 16];
  for x in 0..16 {
    slice[x] = (get_u64() & 0xFF) as u8;
  }
  u128::from_le_bytes(slice)
}

pub fn get_u64() -> u64 {
  if !crate::bindriver::cpu::has_rdrand() {
    // i just use mutex so it works
    unsafe{rng.force_unlock()};
    rng.lock().next_u64()
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
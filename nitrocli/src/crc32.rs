// crc32.rs

// *************************************************************************
// * Copyright (C) 2017 Daniel Mueller (deso@posteo.net)                   *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

/// Polynomial used in STM32.
const CRC32_POLYNOMIAL: u32 = 0x04c11db7;


fn crc32(mut crc: u32, data: u32) -> u32 {
  crc = crc ^ data;

  for _ in 0..32 {
    if crc & 0x80000000 != 0 {
      crc = (crc << 1) ^ CRC32_POLYNOMIAL;
    } else {
      crc = crc << 1;
    }
  }
  return crc;
}


/// Retrieve a u32 slice of the 'data' part.
///
/// Note that the size of the supplied data has to be a multiple of 4
/// bytes.
fn as_slice_u32(data: &[u8]) -> &[u32] {
  assert!(data.len() % ::std::mem::size_of::<u32>() == 0);

  unsafe {
    let ptr = data.as_ptr() as *const u32;
    let len = data.len() / ::std::mem::size_of::<u32>();
    return ::std::slice::from_raw_parts(ptr, len);
  }
}


/// Calculate the CRC of a byte slice.
pub fn crc(data: &[u8]) -> u32 {
  let mut crc = 0xffffffff;
  let data = as_slice_u32(data);

  for i in 0..data.len() {
    crc = crc32(crc, data[i]);
  }
  return crc;
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_crc32() {
    let mut crc = 0;

    // The expected values were computed with the original function.
    crc = crc32(crc, 0xdeadbeef);
    assert_eq!(crc, 0x46dec763);

    crc = crc32(crc, 42);
    assert_eq!(crc, 0x7e579b45);
  }

  #[test]
  fn test_crc() {
    let data = &"thisisatextthatistobecrced..".to_string().into_bytes();
    let crc = crc(data);

    assert_eq!(crc, 0x469db4ee);
  }
}

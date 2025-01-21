use byteorder::{BigEndian, LittleEndian};
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::fs::File;
use std::io::{Cursor, Read, Write};

fn parity_bit(bytes: &[u8]) -> u8 {
    let mut n_ones = 0;
    for byte in bytes {
        n_ones += byte.count_ones();
        println!("{} (0b{:08b}) has {} one bits", byte, byte, byte.count_ones());
    }

    (n_ones % 2 == 0) as u8
}


fn main() {
    let abc = b"abc";
    println!("input: {:?}", abc);
    println!("output: {:08x}", parity_bit(abc));
    println!();
    let abcd = b"abcd";
    println!("input: {:?}", abcd);
    println!("result: {:08x}", parity_bit(abcd))
  }

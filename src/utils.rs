// Bytes to int conversion.
//
// -------------------------------------------------------------------------------
//                              BIG AND LITTLE ENDIAN CONVERSION
// -------------------------------------------------------------------------------

// TODO: These need to be written as macros (Getting very repetitive).

// BIG ENDIAN

use std::{
    error::Error,
    fs::File,
    io::{Read, Seek},
};

pub trait FromBe {
    // N represents size in byte.
    const N: usize;

    fn be_from_slice(buf: &[u8]) -> Self;
}

impl FromBe for u8 {
    const N: usize = std::mem::size_of::<u8>();

    fn be_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u8 as FromBe>::N] = buf.try_into().unwrap();
        Self::from_be_bytes(byte_slice)
    }
}

impl FromBe for u16 {
    const N: usize = std::mem::size_of::<u16>();

    fn be_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u16 as FromBe>::N] = buf.try_into().unwrap();
        Self::from_be_bytes(byte_slice)
    }
}

impl FromBe for u32 {
    const N: usize = std::mem::size_of::<u32>();

    fn be_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u32 as FromBe>::N] = buf.try_into().unwrap();
        Self::from_be_bytes(byte_slice)
    }
}

impl FromBe for u64 {
    const N: usize = std::mem::size_of::<u64>();

    fn be_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u64 as FromBe>::N] = buf.try_into().unwrap();
        Self::from_be_bytes(byte_slice)
    }
}

pub fn parse_be_byte_to_int<T: FromBe>(buf: &[u8], start_byte: usize) -> T {
    T::be_from_slice(&buf[start_byte..start_byte + T::N])
}

// LITTLE ENDIAN

pub trait FromLe {
    // N represents size in byte.
    const N: usize;

    fn le_from_slice(buf: &[u8]) -> Self;
}

impl FromLe for u8 {
    const N: usize = std::mem::size_of::<u8>();

    fn le_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u8 as FromLe>::N] = buf.try_into().unwrap();
        Self::from_le_bytes(byte_slice)
    }
}

impl FromLe for u16 {
    const N: usize = std::mem::size_of::<u16>();

    fn le_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u16 as FromLe>::N] = buf.try_into().unwrap();
        Self::from_le_bytes(byte_slice)
    }
}

impl FromLe for u32 {
    const N: usize = std::mem::size_of::<u32>();

    fn le_from_slice(buf: &[u8]) -> Self {
        let byte_slice: [u8; <u32 as FromLe>::N] = buf.try_into().unwrap();
        Self::from_le_bytes(byte_slice)
    }
}

pub fn parse_le_byte_to_int<T: FromLe>(buf: &[u8], start_byte: usize) -> T {
    T::le_from_slice(&buf[start_byte..start_byte + T::N])
}

// -------------------------------------------------------------------------------
//                          BIT MANIPULATION
// -------------------------------------------------------------------------------

fn is_msb_negative(num: u8) -> bool {
    (num & 0x80) != 0
}

// -------------------------------------------------------------------------------
//                          VARINT PARSING
// -------------------------------------------------------------------------------

// Returns size of bits
pub fn parse_varint_to_int(buf: &[u8], _result: &mut u64) -> usize {
    let mut idx: usize = 0;
    let mut varint: u64 = 0;

    while is_msb_negative(buf[idx]) {
        // clearing the MSB, since only last 7 bits are data.
        // 0x7F = 0111 1111
        let cur_num = buf[idx] & 0x7F;
        varint = varint | (cur_num as u64);
        if idx < 8 {
            varint = varint << 7;
        } else {
            varint = varint << 8;
        }
        idx += 1;
    }

    if idx == 8 {
        varint = varint | (buf[idx] as u64);
    } else {
        varint = varint | ((buf[idx] & 0x7F) as u64);
    }

    *_result = varint;

    // We are return +1 cause, we are starting the index from 0
    idx + 1
}

// pub fn get_parse_varint_to_int(buf: &[u8]) -> u64 {
//     let mut result: u64 = 0;
//     let _ = parse_varint_to_int(buf, &mut result);
//
//     result
// }

pub fn convert_u8_to_u16_le(buf: &[u8]) -> Box<[u16]> {
    let res = buf
        .chunks(2)
        .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()));
    res.collect()
}

pub fn convert_u8_to_u16_be(buf: &[u8]) -> Box<[u16]> {
    let res = buf
        .chunks(2)
        .map(|chunk| u16::from_be_bytes(chunk.try_into().unwrap()));
    res.collect()
}

pub fn read_specific_bytes(
    filepath: &String,
    start_byte: u16,
    pg_size: u16,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut file = File::open(filepath)?;

    let mut buffer = vec![0u8; pg_size as usize];
    file.seek(std::io::SeekFrom::Start((start_byte) as u64))?;

    file.read(&mut buffer)?;

    Ok(buffer)
}

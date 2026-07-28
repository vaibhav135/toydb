use std::{
    error::Error,
    fs::File,
    io::{Read, Seek},
};

// BIG ENDIAN
#[macro_export]
macro_rules! parse_be_byte_to_int {
    ($buf:expr, $start_byte:expr, $size:ident) => {{
        const N: usize = std::mem::size_of::<$size>();

        let byte_slice: [u8; N] = $buf[$start_byte..$start_byte + N].try_into().unwrap();
        $size::from_be_bytes(byte_slice)
    }};
}

// LITTLE ENDIAN
#[macro_export]
macro_rules! parse_le_byte_to_int {
    ($buf:expr, $start_byte:expr, $size:ident) => {{
        const N: usize = std::mem::size_of::<$size>();

        let byte_slice: [u8; N] = $buf[$start_byte..$start_byte + N].try_into().unwrap();
        $size::from_le_bytes(byte_slice)
    }};
}

use crate::file::enums::TxtEncoding;

pub(super) use super::parse_be_byte_to_int;
pub(super) use super::parse_le_byte_to_int;

// -------------------------------------------------------------------------------
//                          BIT MANIPULATION
// -------------------------------------------------------------------------------

pub fn is_msb_negative(num: u8) -> bool {
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

pub fn get_enconding_type(encoding_val: u32) -> TxtEncoding {
    let encoding_type = <u32 as Into<TxtEncoding>>::into(encoding_val);
    encoding_type
}

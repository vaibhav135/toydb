use std::error::Error;

use crate::{
    file::enums::TxtEncoding,
    utils::{
        convert_u8_to_u16_be, convert_u8_to_u16_le, get_parse_varint_to_int, parse_be_byte_to_int,
        parse_varint_to_int,
    },
};

#[derive(Debug)]
pub enum RecordDataType {
    STR(String),
    INT(u64),
    FLOAT(f64),
    BLOB(Box<[u8]>),
    NULL,
}

#[derive(Debug)]
pub struct RecordFormat {
    header_size: u64,
    num_of_cols: u16,
    rows: Vec<(u64, usize, RecordDataType)>, // Vec of (serial type , content size, data)
}

// Record contains header and body in this order -> [header, body]
impl RecordFormat {
    pub fn new() -> Self {
        RecordFormat {
            header_size: 0,
            num_of_cols: 0,
            rows: vec![],
        }
    }

    fn parse_string_payload(
        &self,
        buf: &[u8],
        encoding_type: TxtEncoding,
    ) -> Result<String, Box<dyn Error>> {
        let content;

        match encoding_type {
            TxtEncoding::UTF16BE => {
                let buf_16 = convert_u8_to_u16_be(buf);
                content = String::from_utf16(&buf_16)?;
            }
            TxtEncoding::UTF16LE => {
                let buf_16 = convert_u8_to_u16_le(buf);
                content = String::from_utf16(&buf_16)?;
            }
            _ => {
                content = String::from_utf8(buf.to_vec())?;
            }
        }

        Ok(content)
    }

    fn get_content_size_for_stype(&self, stype: u64) -> usize {
        match stype {
            5 => 6,
            6 | 7 => 8,
            8 | 9 | 12 | 13 => 0,
            _ => {
                if stype < 5 {
                    return stype as usize;
                }

                let is_stype_even = stype % 2 == 0;

                if stype > 12 && is_stype_even {
                    return ((stype - 12) / 2) as usize;
                } else if stype > 13 && !is_stype_even {
                    return ((stype - 13) / 2) as usize;
                }

                0
            }
        }
    }

    pub fn set_records(
        &mut self,
        buf: &[u8],
        mut cur_cell_offset: usize,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // 1st byte of header is the size of the header itself.
        let mut header_size = 0;
        let header_varint_size = parse_varint_to_int(&buf[cur_cell_offset..], &mut header_size);

        let mut header_idx = header_size - 1;
        cur_cell_offset += header_varint_size;

        // (u64, usize, RecordFormat) = (serial type, content size, content)
        let mut payload_data: Vec<(u64, usize, RecordDataType)> = Vec::new();

        let mut content_start_idx = cur_cell_offset + (header_size as usize) - 1;
        let mut content_end_idx = cur_cell_offset + (header_size as usize) - 1;

        while header_idx > 0 {
            // stype = serial type.
            let mut stype = 0;

            // Usually the serial type size will be 1 varint but large strings and
            // BLOB are the only exception which might extend to more that 1 byte
            // varints.
            let stype_varint_size = parse_varint_to_int(&buf[cur_cell_offset..], &mut stype);

            header_idx -= stype_varint_size as u64;

            // Usually the serial type varint size are 1 but only in case of BLOB or string they
            // might actually exceed.
            cur_cell_offset += stype_varint_size;

            let content_size = self.get_content_size_for_stype(stype);

            content_end_idx += content_size;
            let buf_slice = &buf[content_start_idx..content_end_idx];

            let content: RecordDataType = match stype {
                0 => RecordDataType::NULL,
                1..=6 => {
                    let val = get_parse_varint_to_int(buf_slice);

                    RecordDataType::INT(val)
                }
                7 => {
                    let val = f64::from_be_bytes(buf_slice.try_into()?);
                    RecordDataType::FLOAT(val)
                }
                10..=11 => RecordDataType::NULL,
                8 | 9 | 12 | 13 => RecordDataType::INT(0),
                _ => {
                    if stype > 12 && (stype % 2) == 0 {
                        let blob = Box::from(buf_slice);
                        RecordDataType::BLOB(blob)
                    } else {
                        let encoding_type = get_enconding_type(buf);
                        let utf_content = self.parse_string_payload(buf_slice, encoding_type)?;
                        // println!("Content: {}", utf_content);
                        RecordDataType::STR(utf_content)
                    }
                }
            };

            payload_data.push((stype, content_size, content));

            content_start_idx += content_size;
        }

        self.header_size = header_size;
        self.num_of_cols = (header_size - 1) as u16;
        self.rows = payload_data;

        Ok(cur_cell_offset)
    }
}

pub fn get_enconding_type(buf: &[u8]) -> TxtEncoding {
    // text encoding is a  4-byte BE int at offset 56 -> https://www.sqlite.org/fileformat2.html#enc
    let encoding_val = parse_be_byte_to_int::<u32>(buf, 56);
    let encoding_type = <u32 as Into<TxtEncoding>>::into(encoding_val);
    // println!("Encoding Val: {}", encoding_val);
    // println!("Encoding Type: {:?}", encoding_type);
    encoding_type
}

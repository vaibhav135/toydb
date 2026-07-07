use std::{error::Error, iter::Enumerate};

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
    pub header_size: u64,
    pub num_of_cols: u16,
    pub rows: Vec<Row>,
}

#[derive(Debug)]
pub struct Row {
    // This will have serial type, content size
    pub header: (u64, usize),
    // Reason to make it optional is that, if the content overflows then the current cell row might
    // only has 1 datatype of even a part of that data. In that case we have the overflow page
    // which holds the rest of the data and is a linked list btw.
    pub content: RecordDataType,
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
        encoding_type: &TxtEncoding,
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
        encoding_type: &TxtEncoding,
        // mut cur_cell_offset: usize,
        cont_payload_idx: &mut u32,
        cont_remain_bytes: &mut u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1st byte of header is the size of the header itself.
        let mut header_size = 0;
        parse_varint_to_int(&buf, &mut header_size);

        let mut rows = vec![];

        // (serial type,   content size)
        let mut payload_header: Vec<(u64, usize)> = vec![];

        let mut stype_idx: usize = 1;

        loop {
            if stype_idx >= (header_size - 1) as usize {
                break;
            }

            // stype = serial type.
            let mut stype = 0;

            // Usually the serial type size will be 1 varint but large strings and
            // BLOB are the only exception which might extend to more that 1 byte
            // varints.
            let stype_varint_size = parse_varint_to_int(&buf[stype_idx..], &mut stype);

            stype_idx += stype_varint_size;
            self.num_of_cols += 1;

            let content_size = self.get_content_size_for_stype(stype);

            payload_header.push((stype, content_size));
            rows.push(Row {
                header: (stype, content_size),
                content: RecordDataType::NULL,
            })
        }

        self.header_size = header_size;

        let mut content_rows: Vec<RecordDataType> = vec![];

        self.set_content(
            buf,
            encoding_type,
            cont_payload_idx,
            payload_header,
            &mut content_rows,
            cont_remain_bytes,
        )?;

        for (idx, content) in content_rows.into_iter().enumerate() {
            rows[idx].content = content;
        }

        self.rows = rows;

        Ok(())
    }

    pub fn set_content(
        &mut self,
        buf: &[u8],
        encoding_type: &TxtEncoding,
        cont_payload_idx: &mut u32,
        payload_header: Vec<(u64, usize)>,
        content_rows: &mut Vec<RecordDataType>,
        cont_remain_bytes: &mut u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let buf_len = buf.len();
        let mut content_start_idx = self.header_size as usize;
        let mut content_end_idx = self.header_size as usize;

        for (idx, header) in payload_header.iter().enumerate() {
            let content_size = header.1;
            let stype = header.0;

            if content_end_idx >= buf_len {
                break;
            } else if (content_size + content_end_idx) > buf_len {
                content_end_idx = buf_len;

                let temp_cont_remain_bytes =
                    (content_size - (content_end_idx - content_start_idx)) as u32;

                if temp_cont_remain_bytes > *cont_remain_bytes {
                    *cont_remain_bytes = temp_cont_remain_bytes - *cont_remain_bytes;
                } else {
                    *cont_remain_bytes -= temp_cont_remain_bytes;
                }
            } else {
                content_end_idx += content_size;
            }

            let buf_slice = &buf[content_start_idx..content_end_idx];

            let content = self.get_content(buf_slice, stype, encoding_type)?;

            content_start_idx = content_end_idx;

            content_rows.push(content);
            *cont_payload_idx = idx as u32;
        }

        Ok(())
    }

    pub fn get_content(
        &self,
        buf: &[u8],
        stype: u64,
        encoding_type: &TxtEncoding,
    ) -> Result<RecordDataType, Box<dyn std::error::Error>> {
        let content = match stype {
            0 => RecordDataType::NULL,
            1..=6 => {
                let val = get_parse_varint_to_int(buf);

                RecordDataType::INT(val)
            }
            7 => {
                let val = f64::from_be_bytes(buf.try_into()?);
                RecordDataType::FLOAT(val)
            }
            10..=11 => RecordDataType::NULL,
            8 | 9 | 12 | 13 => RecordDataType::INT(0),
            _ => {
                if stype > 12 && (stype % 2) == 0 {
                    let blob = Box::from(buf);
                    RecordDataType::BLOB(blob)
                } else {
                    let utf_content = self.parse_string_payload(buf, encoding_type)?;
                    RecordDataType::STR(utf_content)
                }
            }
        };

        Ok(content)
    }

    pub fn get_content_len(content: &RecordDataType) -> usize {
        match content {
            RecordDataType::STR(data) => data.bytes().len(),
            // BLOB is already in bytes.
            RecordDataType::BLOB(data) => data.len(),
            RecordDataType::FLOAT(_) | RecordDataType::INT(_) => 8,
            _ => 0,
        }
    }
}

pub fn get_enconding_type(encoding_val: u32) -> TxtEncoding {
    let encoding_type = <u32 as Into<TxtEncoding>>::into(encoding_val);
    encoding_type
}

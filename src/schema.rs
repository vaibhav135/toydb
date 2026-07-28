use std::{error::Error, iter::Enumerate};

use crate::{
    btree::{Root, SchemaType, SqlSchema},
    file::enums::TxtEncoding,
    page::PageHeader,
    utils::{
        convert_u8_to_u16_be, convert_u8_to_u16_le, get_enconding_type, is_msb_negative,
        parse_be_byte_to_int, parse_varint_to_int,
    },
};

/*
*
* Record format
*  format:     [header size,  serial type,  value of each col]
*  datatype:   [varint     ,  varint     ,  this could be N bytes (also this is where we have schema type)]
*
*  for value of the columns for serial type 0, 8, 9, 12 and 13 the value is zero bytes in length. if the value
*  is greater than 12 and is even then it's a BLOB.
*
*           A Blob is a file (only pure bytes right) ofcourse there is overflow and all involved but that an afterthought.
*           As a general concept when you insert an image or audio or anything it's goes as blob
*
*  for value >= 13 and odd it will be a String (could be sql, tablename, name, schema type)
*
* */

#[derive(Debug)]
pub enum RecordDataType {
    STR(String),
    INT(i64),
    FLOAT(f64),
    BLOB(Box<[u8]>),
    NULL,
}

impl Into<String> for &RecordDataType {
    fn into(self) -> String {
        match self {
            RecordDataType::STR(str) => str.to_owned(),
            _ => String::new(),
        }
    }
}

impl Into<Result<SchemaType, Box<dyn Error>>> for &RecordDataType {
    fn into(self) -> Result<SchemaType, Box<dyn Error>> {
        let schema_type: SchemaType = match self {
            RecordDataType::STR(str) => String::try_from(str)?.try_into()?,
            _ => SchemaType::TABLE,
        };

        Ok(schema_type)
    }
}

impl Into<i64> for &RecordDataType {
    fn into(self) -> i64 {
        match self {
            RecordDataType::INT(val) => val.to_owned(),
            _ => 0,
        }
    }
}

fn parse_non_prim_int(buf: &[u8], size: u8) -> i64 {
    let mut msb = 0x00;
    if is_msb_negative(buf[0]) {
        msb = 0xFF;
    }

    let res: i64 = 0;
    match size {
        24 => {
            let new_buf = [msb, buf[0], buf[1], buf[2]];
            parse_be_byte_to_int!(buf, 0, i32) as i64
        }
        _ => {
            let new_buf = [msb, msb, buf[0], buf[1], buf[2], buf[3], buf[4], buf[5]];
            parse_be_byte_to_int!(buf, 0, i64)
        }
    }
}

trait Schema {
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

    fn parse_content(
        &self,
        buf: &[u8],
        stype: u64,
        encoding_type: &TxtEncoding,
    ) -> Result<RecordDataType, Box<dyn std::error::Error>> {
        let content = match stype {
            0 => RecordDataType::NULL,
            1..=6 => {
                let val;

                if stype == 1 {
                    // 8bit 2's complement
                    val = parse_be_byte_to_int!(buf, 0, i8) as i64;
                } else if stype == 2 {
                    // 16 bit 2's complement
                    val = parse_be_byte_to_int!(buf, 0, i16) as i64;
                } else if stype == 3 {
                    // 24 bit 2's complement
                    val = parse_non_prim_int(buf, 24);
                } else if stype == 4 {
                    // 32 bit 2's complement
                    val = parse_be_byte_to_int!(buf, 0, i32) as i64;
                } else if stype == 5 {
                    // 48 bit 2's complement
                    val = parse_non_prim_int(buf, 48)
                } else {
                    // 64 bit 2's complement
                    val = parse_be_byte_to_int!(buf, 0, i64);
                }

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

    // Methods for root node - (Leaf table)
    type SchemaType;
    fn extract_schema(
        &self,
        enc_val: u32,
        payload: &[u8],
    ) -> Result<Self::SchemaType, Box<dyn Error>>;

    fn get_sqlite_schema_str(&self, content: Option<&RecordDataType>) -> String;
    fn get_sqlite_schema_int(&self, content: Option<&RecordDataType>) -> i64;
}

impl Schema for Root {
    type SchemaType = SqlSchema;

    fn get_sqlite_schema_str(&self, content: Option<&RecordDataType>) -> String {
        let schema_str: String = if let Some(res) = content {
            res.into()
        } else {
            String::new()
        };
        schema_str
    }

    fn get_sqlite_schema_int(&self, content: Option<&RecordDataType>) -> i64 {
        let schema_int: i64 = if let Some(res) = content {
            res.into()
        } else {
            0
        };
        schema_int
    }

    fn extract_schema(
        &self,
        enc_val: u32,
        payload: &[u8],
    ) -> Result<Self::SchemaType, Box<dyn Error>> {
        let mut header = 0;
        let header_varint_size = parse_varint_to_int(payload, &mut header);

        let mut cur_payload_offset = header_varint_size;

        let mut stype_with_content_size: Vec<(u64, usize)> = vec![];

        let mut total_cols = header - 1;

        while total_cols > 0 {
            let mut stype = 0u64;

            cur_payload_offset += parse_varint_to_int(&payload[cur_payload_offset..], &mut stype);

            let content_size = self.get_content_size_for_stype(stype);
            stype_with_content_size.push((stype, content_size));
            total_cols -= 1;
        }

        let mut schema_content: Vec<RecordDataType> = vec![];
        for (stype, content_size) in stype_with_content_size {
            let content = self.parse_content(
                &payload[cur_payload_offset..cur_payload_offset + content_size],
                stype,
                &get_enconding_type(enc_val),
            )?;
            schema_content.push(content);
            cur_payload_offset += content_size;
        }

        let schema_type: SchemaType = if let Some(res) = schema_content.get(0) {
            let schema_type_result: Result<SchemaType, Box<dyn Error>> = res.into();
            schema_type_result?
        } else {
            SchemaType::TABLE
        };

        let name: String = self.get_sqlite_schema_str(schema_content.get(1));
        let tbl_name: String = self.get_sqlite_schema_str(schema_content.get(2));
        let rootpg: i64 = self.get_sqlite_schema_int(schema_content.get(3));
        let sql: String = self.get_sqlite_schema_str(schema_content.get(4));

        Ok(SqlSchema {
            schema_type,
            name,
            tbl_name,
            rootpg,
            sql,
        })
    }
}

//
//
// #[derive(Debug)]
// pub struct RecordFormat {
//     pub header_size: u64,
//     pub num_of_cols: u16,
//     pub rows: Vec<Row>,
// }
//
// #[derive(Debug)]
// pub struct Row {
//     // This will have serial type, content size
//     pub header: (u64, usize),
//     // Reason to make it optional is that, if the content overflows then the current cell row might
//     // only has 1 datatype of even a part of that data. In that case we have the overflow page
//     // which holds the rest of the data and is a linked list btw.
//     pub content: RecordDataType,
// }
//
// // Record contains header and body in this order -> [header, body]
// impl RecordFormat {
//     pub fn new() -> Self {
//         RecordFormat {
//             header_size: 0,
//             num_of_cols: 0,
//             rows: vec![],
//         }
//     }
//
//
//     fn get_content_size_for_stype(&self, stype: u64) -> usize {
//         match stype {
//             5 => 6,
//             6 | 7 => 8,
//             8 | 9 | 12 | 13 => 0,
//             _ => {
//                 if stype < 5 {
//                     return stype as usize;
//                 }
//
//                 let is_stype_even = stype % 2 == 0;
//
//                 if stype > 12 && is_stype_even {
//                     return ((stype - 12) / 2) as usize;
//                 } else if stype > 13 && !is_stype_even {
//                     return ((stype - 13) / 2) as usize;
//                 }
//
//                 0
//             }
//         }
//     }
//
//     pub fn set_records(
//         &mut self,
//         buf: &[u8],
//         encoding_type: &TxtEncoding,
//         // mut cur_cell_offset: usize,
//         cont_payload_idx: &mut u32,
//         cont_remain_bytes: &mut u32,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         // 1st byte of header is the size of the header itself.
//         let mut header_size = 0;
//         parse_varint_to_int(&buf, &mut header_size);
//
//         let mut rows = vec![];
//
//         // (serial type,   content size)
//         let mut payload_header: Vec<(u64, usize)> = vec![];
//
//         let mut stype_idx: usize = 1;
//
//         loop {
//             if stype_idx >= (header_size - 1) as usize {
//                 break;
//             }
//
//             // stype = serial type.
//             let mut stype = 0;
//
//             // Usually the serial type size will be 1 varint but large strings and
//             // BLOB are the only exception which might extend to more that 1 byte
//             // varints.
//             let stype_varint_size = parse_varint_to_int(&buf[stype_idx..], &mut stype);
//
//             stype_idx += stype_varint_size;
//             self.num_of_cols += 1;
//
//             let content_size = self.get_content_size_for_stype(stype);
//
//             payload_header.push((stype, content_size));
//             rows.push(Row {
//                 header: (stype, content_size),
//                 content: RecordDataType::NULL,
//             })
//         }
//
//         self.header_size = header_size;
//
//         let mut content_rows: Vec<RecordDataType> = vec![];
//
//         self.set_content(
//             buf,
//             encoding_type,
//             cont_payload_idx,
//             payload_header,
//             &mut content_rows,
//             cont_remain_bytes,
//         )?;
//
//         for (idx, content) in content_rows.into_iter().enumerate() {
//             rows[idx].content = content;
//         }
//
//         self.rows = rows;
//
//         Ok(())
//     }
//
//     pub fn set_content(
//         &mut self,
//         buf: &[u8],
//         encoding_type: &TxtEncoding,
//         cont_payload_idx: &mut u32,
//         payload_header: Vec<(u64, usize)>,
//         content_rows: &mut Vec<RecordDataType>,
//         cont_remain_bytes: &mut u32,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         let buf_len = buf.len();
//         let mut content_start_idx = self.header_size as usize;
//         let mut content_end_idx = self.header_size as usize;
//
//         for (idx, header) in payload_header.iter().enumerate() {
//             let content_size = header.1;
//             let stype = header.0;
//
//             if content_end_idx >= buf_len {
//                 break;
//             } else if (content_size + content_end_idx) > buf_len {
//                 content_end_idx = buf_len;
//
//                 let temp_cont_remain_bytes =
//                     (content_size - (content_end_idx - content_start_idx)) as u32;
//
//                 if temp_cont_remain_bytes > *cont_remain_bytes {
//                     *cont_remain_bytes = temp_cont_remain_bytes - *cont_remain_bytes;
//                 } else {
//                     *cont_remain_bytes -= temp_cont_remain_bytes;
//                 }
//             } else {
//                 content_end_idx += content_size;
//             }
//
//             let buf_slice = &buf[content_start_idx..content_end_idx];
//
//             let content = self.get_content(buf_slice, stype, encoding_type)?;
//
//             content_start_idx = content_end_idx;
//
//             content_rows.push(content);
//             *cont_payload_idx = idx as u32;
//         }
//
//         Ok(())
//     }
//
//     pub fn get_content(
//         &self,
//         buf: &[u8],
//         stype: u64,
//         encoding_type: &TxtEncoding,
//     ) -> Result<RecordDataType, Box<dyn std::error::Error>> {
//         let content = match stype {
//             0 => RecordDataType::NULL,
//             1..=6 => {
//                 let val = get_parse_varint_to_int(buf);
//
//                 RecordDataType::INT(val)
//             }
//             7 => {
//                 let val = f64::from_be_bytes(buf.try_into()?);
//                 RecordDataType::FLOAT(val)
//             }
//             10..=11 => RecordDataType::NULL,
//             8 | 9 | 12 | 13 => RecordDataType::INT(0),
//             _ => {
//                 if stype > 12 && (stype % 2) == 0 {
//                     let blob = Box::from(buf);
//                     RecordDataType::BLOB(blob)
//                 } else {
//                     let utf_content = self.parse_string_payload(buf, encoding_type)?;
//                     RecordDataType::STR(utf_content)
//                 }
//             }
//         };
//
//         Ok(content)
//     }
//
//     pub fn get_content_len(content: &RecordDataType) -> usize {
//         match content {
//             RecordDataType::STR(data) => data.bytes().len(),
//             // BLOB is already in bytes.
//             RecordDataType::BLOB(data) => data.len(),
//             RecordDataType::FLOAT(_) | RecordDataType::INT(_) => 8,
//             _ => 0,
//         }
//     }
// }
//

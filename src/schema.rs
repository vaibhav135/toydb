use std::error::Error;

use crate::{
    btree::{Child, Root, SchemaType, SqlSchema},
    file::enums::TxtEncoding,
    record_type::RecordDataType,
    utils::{
        convert_u8_to_u16_be, convert_u8_to_u16_le, get_enconding_type, is_msb_negative,
        parse_be_byte_to_int, parse_varint_to_int,
    },
};

fn parse_non_prim_int(buf: &[u8], size: u8) -> RecordDataType {
    let mut msb = 0x00;
    if is_msb_negative(buf[0]) {
        msb = 0xFF;
    }

    match size {
        24 => {
            let new_buf = [msb, buf[0], buf[1], buf[2]];
            RecordDataType::INT32(parse_be_byte_to_int!(buf, 0, i32))
        }
        _ => {
            let new_buf = [msb, msb, buf[0], buf[1], buf[2], buf[3], buf[4], buf[5]];
            RecordDataType::INT64(parse_be_byte_to_int!(buf, 0, i64))
        }
    }
}

pub trait Schema {
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
                if stype == 1 {
                    // 8bit 2's complement
                    return Ok(RecordDataType::INT8(parse_be_byte_to_int!(buf, 0, i8)));
                } else if stype == 2 {
                    // 16 bit 2's complement
                    return Ok(RecordDataType::INT16(parse_be_byte_to_int!(buf, 0, i16)));
                } else if stype == 3 {
                    // 24 bit 2's complement
                    return Ok(parse_non_prim_int(buf, 24));
                } else if stype == 4 {
                    // 32 bit 2's complement
                    return Ok(RecordDataType::INT32(parse_be_byte_to_int!(buf, 0, i32)));
                } else if stype == 5 {
                    // 48 bit 2's complement
                    return Ok(parse_non_prim_int(buf, 48));
                } else {
                    // 64 bit 2's complement
                    return Ok(RecordDataType::INT64(parse_be_byte_to_int!(buf, 0, i64)));
                }
            }
            7 => {
                let val = f64::from_be_bytes(buf.try_into()?);
                RecordDataType::FLOAT(val)
            }
            10..=11 => RecordDataType::NULL,
            8 | 12 | 13 => RecordDataType::INT8(0),
            9 => RecordDataType::INT8(1),
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

    fn read_content(
        &self,
        enc_val: u32,
        payload: &[u8],
    ) -> Result<Vec<RecordDataType>, Box<dyn Error>> {
        let mut header = 0;
        let header_varint_size = parse_varint_to_int(payload, &mut header);

        let mut cur_payload_offset = header_varint_size;

        let mut stype_with_content_size: Vec<(u64, usize)> = vec![];

        let total_cols = header - header_varint_size as u64;
        let mut cur_col = 0;

        while cur_col < total_cols {
            let mut stype = 0u64;

            let stype_varint_size = parse_varint_to_int(&payload[cur_payload_offset..], &mut stype);
            cur_payload_offset += stype_varint_size;

            let content_size = self.get_content_size_for_stype(stype);
            stype_with_content_size.push((stype, content_size));
            cur_col += stype_varint_size as u64;
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

        Ok(schema_content)
    }
}

pub trait RootSchema: Schema {
    type SchemaType;
    fn extract_schema(
        &self,
        enc_val: u32,
        payload: &[u8],
    ) -> Result<Self::SchemaType, Box<dyn Error>>;

    fn get_sqlite_schema_str(&self, content: Option<&RecordDataType>) -> String;
    fn get_sqlite_schema_int(&self, content: Option<&RecordDataType>) -> u32;
}

impl Schema for Root {}

impl RootSchema for Root {
    type SchemaType = SqlSchema;

    fn get_sqlite_schema_str(&self, content: Option<&RecordDataType>) -> String {
        let schema_str: String = if let Some(res) = content {
            res.into()
        } else {
            String::new()
        };
        schema_str
    }

    fn get_sqlite_schema_int(&self, content: Option<&RecordDataType>) -> u32 {
        let schema_int: u32 = if let Some(res) = content {
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
        let schema_content = self.read_content(enc_val, payload)?;

        let schema_type: SchemaType = if let Some(res) = schema_content.get(0) {
            let schema_type_result: Result<SchemaType, Box<dyn Error>> = res.into();
            schema_type_result?
        } else {
            SchemaType::TABLE
        };

        let name: String = self.get_sqlite_schema_str(schema_content.get(1));
        let tbl_name: String = self.get_sqlite_schema_str(schema_content.get(2));
        let rootpg: u32 = self.get_sqlite_schema_int(schema_content.get(3));
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

impl Schema for Child {}

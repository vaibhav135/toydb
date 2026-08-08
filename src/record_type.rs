use std::{error::Error, ops::Deref};

use crate::btree::SchemaType;

/*
*
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

#[derive(Debug, Clone)]
pub enum RecordDataType {
    STR(String),
    INT8(i8),
    INT16(i16),
    INT32(i32),
    INT64(i64),
    FLOAT(f64),
    BLOB(Box<[u8]>),
    NULL,
}

impl Default for RecordDataType {
    fn default() -> Self {
        Self::STR(String::from(""))
    }
}

impl RecordDataType {
    pub fn is_record_int(record: &RecordDataType) -> bool {
        match record {
            RecordDataType::INT8(_)
            | RecordDataType::INT16(_)
            | RecordDataType::INT32(_)
            | RecordDataType::INT64(_) => true,
            _ => false,
        }
    }

    pub fn is_record_string(record: &RecordDataType) -> bool {
        match record {
            RecordDataType::STR(_) => true,
            _ => false,
        }
    }

    pub fn is_record_float(record: &RecordDataType) -> bool {
        match record {
            RecordDataType::FLOAT(_) => true,
            _ => false,
        }
    }

    pub fn is_record_blob(record: &RecordDataType) -> bool {
        match record {
            RecordDataType::BLOB(_) => true,
            _ => false,
        }
    }

    // Compare types of the two RecordDataType if they are same or not.
    pub fn cmp(val1: &RecordDataType, val2: &RecordDataType) -> bool {
        if RecordDataType::is_record_int(val1) == RecordDataType::is_record_int(val2) {
            return true;
        } else if RecordDataType::is_record_string(&val1) == RecordDataType::is_record_string(val2)
        {
            return true;
        } else if RecordDataType::is_record_float(val1) == RecordDataType::is_record_float(val2) {
            return true;
        } else if RecordDataType::is_record_blob(val1) == RecordDataType::is_record_blob(val2) {
            return true;
        } else {
            return false;
        }
    }

    pub fn convert_str_to_recordformat(input: String) -> RecordDataType {
        let mut res = RecordDataType::STR((&input).to_string());

        if input.parse::<f64>().is_ok().into() {
            res = RecordDataType::FLOAT(input.parse::<f64>().unwrap());
        } else if input.parse::<i64>().is_ok().into() {
            res = RecordDataType::INT64(input.parse::<i64>().unwrap());
        }

        res
    }
}

// ------------ INTO CONVERSION -------------------------------

// TODO: This seems very repetitive. Maybe use a better approach here.

impl Into<String> for &RecordDataType {
    fn into(self) -> String {
        match self {
            RecordDataType::STR(str) => str.to_owned(),
            _ => String::new(),
        }
    }
}

impl Into<String> for RecordDataType {
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
            RecordDataType::STR(str) => SchemaType::try_from(str.to_string())?,
            _ => SchemaType::TABLE,
        };

        Ok(schema_type)
    }
}

impl Into<u64> for &RecordDataType {
    fn into(self) -> u64 {
        match self {
            RecordDataType::INT8(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            RecordDataType::INT16(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            RecordDataType::INT32(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            RecordDataType::INT64(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            _ => 0,
        }
    }
}

impl Into<u64> for RecordDataType {
    fn into(self) -> u64 {
        match self {
            RecordDataType::INT8(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            RecordDataType::INT16(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            RecordDataType::INT32(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            RecordDataType::INT64(val) => val.to_owned().try_into().unwrap_or(0) as u64,
            _ => 0,
        }
    }
}

impl Into<i64> for &RecordDataType {
    fn into(self) -> i64 {
        match self {
            RecordDataType::INT8(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            RecordDataType::INT16(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            RecordDataType::INT32(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            RecordDataType::INT64(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            _ => 0,
        }
    }
}

impl Into<i64> for RecordDataType {
    fn into(self) -> i64 {
        match self {
            RecordDataType::INT8(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            RecordDataType::INT16(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            RecordDataType::INT32(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            RecordDataType::INT64(val) => val.to_owned().try_into().unwrap_or(0) as i64,
            _ => 0,
        }
    }
}

impl Into<f64> for &RecordDataType {
    fn into(self) -> f64 {
        match self {
            RecordDataType::FLOAT(val) => val.to_owned(),
            _ => 0.0,
        }
    }
}

impl Into<f64> for RecordDataType {
    fn into(self) -> f64 {
        match self {
            RecordDataType::FLOAT(val) => val.to_owned(),
            _ => 0.0,
        }
    }
}

impl Into<u32> for &RecordDataType {
    fn into(self) -> u32 {
        match self {
            RecordDataType::INT8(val) => val.to_owned() as u32,
            RecordDataType::INT16(val) => val.to_owned() as u32,
            RecordDataType::INT32(val) => val.to_owned() as u32,
            _ => 0,
        }
    }
}

// ------------ MACROS -------------------------------

#[macro_export]
macro_rules! convert_from_record_format_to {
    ($fromval: expr, $totype: ty) => {{
        let toval: $totype = $fromval.to_owned().into();
        toval
    }};
}

pub(super) use super::convert_from_record_format_to;

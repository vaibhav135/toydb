use std::collections::HashMap;

use crate::{btree::InteriorTablePayload, page::PageHeader};

#[derive(Debug)]
pub struct SqlSchema {
    pub schema_type: SchemaType, // could be a table, index, view or trigger
    pub name: String,            // name of the object
    pub tbl_name: String,        // name of table or view the object is associated with
    pub rootpg: i64,
    pub sql: String,
}

#[derive(Debug, PartialEq)]
pub enum SchemaType {
    TABLE,
    INDEX,
    VIEW,
    TRIGGER,
}

impl TryFrom<String> for SchemaType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let match_value = value.as_str();
        match match_value {
            "table" => Ok(SchemaType::TABLE),
            "index" => Ok(SchemaType::INDEX),
            "trigger" => Ok(SchemaType::TRIGGER),
            "view" => Ok(SchemaType::VIEW),
            _ => Err(format!("invalid schema type!!!")),
        }
    }
}

#[derive(Debug, Default)]
pub struct DBFileInfo {
    pub total_dbsize: usize,
    pub filepath: String,
}

#[derive(Debug, Default)]
pub struct DBHeader {
    // I have not added all the fields (only the ones I might need for now).
    pub page_size: u16,

    // ff: file format | w: write | r: read
    pub ffw_ver: u8,
    pub ffr_ver: u8,

    pub resrv_bytes_per_pg: u8,

    pub total_freelist_pages: u32,

    // def: default
    pub def_pgcache_size: u32,
    pub enc_val: u32,
}

#[derive(Debug)]
pub enum RootPayload {
    InteriorTable(Vec<InteriorTablePayload>),
    RootLeafTable(Vec<SqlSchema>),
}

impl Default for RootPayload {
    fn default() -> Self {
        RootPayload::RootLeafTable(vec![])
    }
}

#[derive(Debug)]
pub struct RootPage {
    pub pgheader: PageHeader,
    pub payload: RootPayload,
}

// Root is the first page.
#[derive(Debug, Default)]
pub struct Root {
    pub db_header: DBHeader,
    pub total_pages: usize,

    pub pages: Vec<RootPage>,

    // Root usually have tables either interior or leaf.
    pub tables: HashMap<SchemaType, Vec<SqlSchema>>,

    pub metadata: DBFileInfo,
}

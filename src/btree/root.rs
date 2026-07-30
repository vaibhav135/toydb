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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    LeafTable(Vec<SqlSchema>),
}

impl Default for RootPayload {
    fn default() -> Self {
        RootPayload::LeafTable(vec![])
    }
}

// NOTE: I am not super sure about this. A root page can have interior nodes which means
// schema will be spread throughout different pages, therefore will have page header for each
// page, but I don't know if this is the right structure to represent that.
#[derive(Debug)]
pub struct RootPage {
    pub pgheader: PageHeader,
    pub pgno: u16,
}

// Root is the first page.
#[derive(Debug, Default)]
pub struct Root {
    pub db_header: DBHeader,
    pub total_pages: usize,

    pub pages: Vec<RootPage>,

    // Root usually have tables either interior or leaf.
    // The key is the table name actually
    pub tables: HashMap<String, SqlSchema>,

    pub metadata: DBFileInfo,
}

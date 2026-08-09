/**
*  List of imp links:
*     
*     doc on sqlite limits -> https://sqlite.org/limits.html
*
* */
use std::{collections::HashMap, error::Error};

use crate::{
    btree::InteriorTablePayload,
    page::{Page, PageHeader},
    parse_be_byte_to_int,
};

#[derive(Debug)]
pub struct SqlSchema {
    pub schema_type: SchemaType, // could be a table, index, view or trigger
    pub name: String,            // name of the object
    pub tbl_name: String,        // name of table or view the object is associated with
    pub rootpg: u32,
    pub sql: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaType {
    #[default]
    TABLE,
    INDEX,
    VIEW,
    TRIGGER,
}

impl TryFrom<String> for SchemaType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        // let match_value = value.as_str();

        match value.to_lowercase().as_str() {
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

// NOTE: I am not sure about this. A root page can have interior nodes which means
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
    pub tables: HashMap<String, Vec<SqlSchema>>,

    pub metadata: DBFileInfo,
}

impl Root {
    pub fn read_root_data(
        &self,
        start_offset: u16,
        pgno: u16,
        filepath: &String,
        dbheader: &DBHeader,
        rootpg_list: &mut Vec<RootPage>,
        tables: &mut HashMap<String, Vec<SqlSchema>>,
    ) -> Result<(), Box<dyn Error>> {
        println!("\npg no: {pgno}");

        let pgsize = dbheader.page_size;
        let pgoffset = if pgno > 0 {
            (pgno - 1) as u32 * pgsize as u32
        } else {
            start_offset as u32
        };

        let (pgheader, cells) = self.read_page(filepath, &dbheader, start_offset, pgoffset)?;

        let pgdata = self.get_pgdata(&dbheader, &pgheader, &cells)?;

        rootpg_list.push(RootPage {
            pgheader,
            pgno: pgno,
        });

        match pgdata {
            RootPayload::InteriorTable(payload) => {
                // println!("interior table: {:?}", payload);
                for (idx, item) in payload.iter().enumerate() {
                    self.read_root_data(
                        0,
                        item.leftptr as u16,
                        filepath,
                        dbheader,
                        rootpg_list,
                        tables,
                    )?;

                    if idx == payload.len() - 1 {
                        self.read_root_data(
                            0,
                            item.rightptr as u16,
                            filepath,
                            dbheader,
                            rootpg_list,
                            tables,
                        )?;
                    }
                }
            }
            RootPayload::LeafTable(sqlschema_list) => {
                for schema in sqlschema_list {
                    if tables.contains_key(&schema.tbl_name) {
                        tables
                            .entry(schema.tbl_name.to_owned())
                            .and_modify(|schema_list| schema_list.push(schema));
                    } else {
                        tables.insert(schema.tbl_name.to_string(), vec![schema]);
                    }
                }
            } // _ => Err(format!("Invalid root payload type...")),
        }

        Ok(())
    }

    pub fn read_db_header(&mut self, buf: &[u8]) -> DBHeader {
        // First 100 bytes of the 1st page is database header, and
        // that is where we are extracting page size from.
        let page_size = parse_be_byte_to_int!(buf, 16, u16);

        let resrv_bytes_per_pg = parse_be_byte_to_int!(buf, 20, u8);

        let ffw_ver = parse_be_byte_to_int!(buf, 18, u8);
        let ffr_ver = parse_be_byte_to_int!(buf, 19, u8);

        let total_freelist_pages = parse_be_byte_to_int!(&buf, 36, u32);
        let def_pgcache_size = parse_be_byte_to_int!(buf, 48, u32);

        // text encoding is a  4-byte BE int at offset 56 -> https://www.sqlite.org/fileformat2.html#enc
        let enc_val = parse_be_byte_to_int!(buf, 56, u32);

        // self.total_pages = total_pages;

        DBHeader {
            page_size,
            resrv_bytes_per_pg,
            ffw_ver,
            ffr_ver,
            total_freelist_pages,
            def_pgcache_size,
            enc_val,
        }
    }
}

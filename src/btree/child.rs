use std::collections::HashMap;

use crate::{btree::InteriorTablePayload, page::PageHeader, schema::RecordDataType};

#[derive(Debug)]
pub struct InteriorIndexPayload {
    pub ptr: u32,
    pub data: Option<Vec<RecordDataType>>,
}

#[derive(Debug, Default)]
// Btw root also have leaf table, but since the structure is already
// defined i.e., sql schema. So create a more rigid struct there.
// Also since all we need is the list of data for leaf which is common for
// both the leaf index and leaf table.
pub struct LeafPayload {
    pub rowid: Option<u64>,
    pub data: Vec<RecordDataType>,
}

#[derive(Debug)]
pub enum ChildPayload {
    InteriorTablePayload(Vec<InteriorTablePayload>),
    InteriorIndexPayload(Vec<InteriorIndexPayload>),
    LeafTablePayload(Vec<LeafPayload>),
    LeafIndexPayload(Vec<LeafPayload>),
}

impl Default for ChildPayload {
    fn default() -> Self {
        ChildPayload::LeafTablePayload(vec![])
    }
}

#[derive(Debug, Default)]
pub struct Child {
    pub pgheader: PageHeader,
    pub pgno: u32,

    // HashMap<Table Name: String, List of records (as record itself is an array of fields)
    pub rows: HashMap<String, Vec<Vec<RecordDataType>>>,
}

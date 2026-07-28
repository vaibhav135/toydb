use crate::{btree::InteriorTablePayload, page::PageHeader, schema::RecordDataType};

#[derive(Debug)]
pub struct InteriorIndexPayload {
    ptr: Vec<u32>,
    data: Vec<RecordDataType>,
}

#[derive(Debug, Default)]
// Btw root also have leaf table, but since the structure is already
// defined i.e., sql schema. So create a more rigid struct there.
// Also since all we need is the list of data for leaf which is common for
// both the leaf index and leaf table.
pub struct LeafPayload {
    data: Vec<RecordDataType>,
}

#[derive(Debug)]
pub enum ChildPayload {
    InteriorTablePayload(InteriorTablePayload),
    InteriorIndexPayload(InteriorIndexPayload),
    LeafTablePayload(LeafPayload),
    LeafIndexPayload(LeafPayload),
}

impl Default for ChildPayload {
    fn default() -> Self {
        ChildPayload::LeafTablePayload(LeafPayload { data: vec![] })
    }
}

#[derive(Debug, Default)]
pub struct Child {
    pub pgheader: PageHeader,
    pub pgno: u32,
    pub payload: ChildPayload,
}

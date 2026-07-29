#[derive(Debug)]
pub struct InteriorTablePayload {
    pub ptr: u32,
    pub key: Option<u64>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BTreePageHeaderFormat {
    InteriorIndexBTreePage,
    InteriorTableBTreePage,
    LeafIndexBTreePage,
    LeafTableBTreePage,
}

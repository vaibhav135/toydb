#[derive(Debug)]
pub struct InteriorTablePayload {
    pub leftptr: u32,
    pub rightptr: u32,
    pub key: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BTreePageHeaderFormat {
    InteriorIndexBTreePage,
    InteriorTableBTreePage,
    LeafIndexBTreePage,
    LeafTableBTreePage,
}

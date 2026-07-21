#[derive(Debug)]
pub enum TxtEncoding {
    // 4-byte big-endian integer. With some assigned values.
    UTF8,    // Value = 1
    UTF16LE, // LE = little encoding, Value = 2
    UTF16BE, // BE = big encoding, Value = 3
}

impl From<u32> for TxtEncoding {
    fn from(value: u32) -> Self {
        match value {
            1 => TxtEncoding::UTF8,
            2 => TxtEncoding::UTF16LE,
            _ => TxtEncoding::UTF16LE,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BTreePageHeaderFormat {
    InteriorIndexBTreePage,
    InteriorTableBTreePage,
    LeafIndexBTreePage,
    LeafTableBTreePage,
}

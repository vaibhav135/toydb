pub mod enums;

use super::custom_error::CustomError;

use enums::BTreePageHeaderFormat;

// In the page header format table it's mentioned for which offset
// what is the type of b-tree page we have.
pub fn get_btree_page_type(offset_val: u8) -> Result<BTreePageHeaderFormat, CustomError> {
    match offset_val {
        2 => Ok(BTreePageHeaderFormat::InteriorIndexBTreePage),
        5 => Ok(BTreePageHeaderFormat::InteriorTableBTreePage),
        10 => Ok(BTreePageHeaderFormat::LeafIndexBTreePage),
        13 => Ok(BTreePageHeaderFormat::LeafTableBTreePage),
        _ => Err(CustomError::InvalidOffsetValueError),
    }
}

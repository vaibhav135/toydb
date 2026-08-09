mod child;
mod common;
mod root;

// Root represents cols
// Child represents rows
// Imagine them as nodes. Root at the top, everything else is a child node.

pub use child::{Child, ChildPayload, IndexRow, InteriorIndexPayload, LeafPayload, Row, TableRow};
pub use common::{BTreePageHeaderFormat, InteriorTablePayload};
pub use root::{DBFileInfo, DBHeader, Root, RootPage, RootPayload, SchemaType, SqlSchema};

use crate::record_type::{RecordDataType, convert_from_record_format_to};

/***
*  Return the ptr to the page where the data holding the key (key can be -:
*  rowid, or a indexed value) might be.
*
*  NOTE: The way I have implemented this is very naiive but it's ok, we'll refactor
*  it in the future to make it more effiecient.
* */
#[macro_export]
macro_rules! interior_binsearch {
    ($payload: expr, $seek: expr, $accessor: expr) => {{
        let _interior_binsearch = || -> u32 {
            let mut low = 0;
            let total_len = $payload.len() as isize - 1;
            let mut high = total_len;

            while low <= high {
                let mid = low + (high - low) / 2;
                let mid_usize = mid as usize;
                let cur_elem = &$payload[mid_usize];

                // they both will be equal when only 1 element is left
                if high == low {
                    if $seek <= ($accessor)(cur_elem) {
                        return cur_elem.leftptr;
                    } else {
                        return cur_elem.rightptr;
                    };
                }

                if $seek > ($accessor)(cur_elem) {
                    if mid + 1 >= total_len {
                        if $seek <= ($accessor)(&$payload[(mid + 1) as usize]) {
                            return cur_elem.rightptr;
                        } else {
                            low = mid + 1;
                        }
                    } else {
                        low = mid;
                    }
                } else {
                    if ($accessor)(cur_elem) == $seek {
                        return cur_elem.leftptr;
                    } else if mid - 1 >= 0 {
                        if $seek > ($accessor)(&$payload[(mid - 1) as usize]) {
                            return cur_elem.leftptr;
                        } else if $seek == ($accessor)(&$payload[(mid - 1) as usize]) {
                            return $payload[mid_usize - 1].leftptr;
                        } else {
                            high = mid - 1;
                        }
                    } else {
                        high = mid;
                    }
                }
            }

            0u32
        };

        _interior_binsearch()
    }};
}

#[macro_export]
macro_rules! leaf_binsearch {
    ($payload: expr, $seek: expr, $type: ty, $accessor: expr) => {{
        let _leaf_binsearch = || {
            let mut low = 0;
            let mut high = $payload.len() as isize - 1;

            while low <= high {
                let mid = low + (high - low) / 2;

                let mid_usize = mid as usize;

                // First val is the indexed field, second is the rowid
                let idx_val: $type = ($accessor)(&$payload[mid_usize].clone().into());

                if idx_val == $seek {
                    return Some($payload[mid_usize].clone());
                } else if idx_val < $seek {
                    low = mid + 1; // Discard left half
                } else {
                    high = mid - 1; // Discard right half
                }
            }

            None
        };

        _leaf_binsearch()
    }};
}

pub(super) use super::interior_binsearch;
pub(super) use super::leaf_binsearch;

pub fn leaf_idx_binsearch_by_val(
    payload: Vec<LeafPayload>,
    seek: RecordDataType,
) -> Option<LeafPayload> {
    if RecordDataType::is_record_int(&seek) {
        let seek_int: u64 = seek.into();
        return leaf_binsearch!(
            payload,
            seek_int,
            u64,
            |p: &LeafPayload| convert_from_record_format_to!(&p.row[0], u64)
        );
    } else if RecordDataType::is_record_string(&seek) {
        println!("\nI am inside the string one....\n\n");
        let seek_str: String = (&seek).into();
        return leaf_binsearch!(
            payload,
            seek_str,
            String,
            |p: &LeafPayload| convert_from_record_format_to!(&p.row[0], String)
        );
    } else if RecordDataType::is_record_float(&seek) {
        let seek_float: f64 = seek.into();
        return leaf_binsearch!(
            payload,
            seek_float,
            f64,
            |p: &LeafPayload| convert_from_record_format_to!(&p.row[0], f64)
        );
    }

    None
}

pub fn interior_idx_binsearch_by_val(
    payload: Vec<InteriorIndexPayload>,
    seek: RecordDataType,
) -> u32 {
    if RecordDataType::is_record_int(&seek) {
        let seek_int: u64 = seek.into();
        return interior_binsearch!(
            payload,
            seek_int,
            |p: &InteriorIndexPayload| convert_from_record_format_to!(
                p.data.as_ref().unwrap()[0],
                u64
            )
        );
    } else if RecordDataType::is_record_string(&seek) {
        let seek_str: String = (&seek).into();
        return interior_binsearch!(
            payload,
            seek_str,
            |p: &InteriorIndexPayload| convert_from_record_format_to!(
                p.data.as_ref().unwrap()[0],
                String
            )
        );
    } else if RecordDataType::is_record_float(&seek) {
        let seek_float: f64 = seek.into();
        return interior_binsearch!(
            payload,
            seek_float,
            |p: &InteriorIndexPayload| convert_from_record_format_to!(
                p.data.as_ref().unwrap()[0],
                f64
            )
        );
    }

    0
}

#[cfg(test)]
mod binsearch_tests {
    use super::*;

    #[test]
    fn test_interior_tbl_binsearch() {
        let payloads = vec![
            InteriorTablePayload {
                leftptr: 100,
                rightptr: 101,
                key: 25,
            },
            InteriorTablePayload {
                leftptr: 101,
                rightptr: 102,
                key: 100,
            },
            InteriorTablePayload {
                leftptr: 102,
                rightptr: 103,
                key: 200,
            },
            InteriorTablePayload {
                leftptr: 103,
                rightptr: 104,
                key: 300,
            },
            InteriorTablePayload {
                leftptr: 104,
                rightptr: 105,
                key: 400,
            },
            InteriorTablePayload {
                leftptr: 105,
                rightptr: 106,
                key: 510,
            },
            InteriorTablePayload {
                leftptr: 106,
                rightptr: 107,
                key: 620,
            },
            InteriorTablePayload {
                leftptr: 107,
                rightptr: 108,
                key: 730,
            },
            InteriorTablePayload {
                leftptr: 108,
                rightptr: 109,
                key: 840,
            },
            InteriorTablePayload {
                leftptr: 109,
                rightptr: 110,
                key: 950,
            },
        ];

        let arr_id_and_res = vec![
            (0, 100),
            (25, 100),
            (26, 101),
            (50, 101),
            (100, 101),
            (101, 102),
            (200, 102),
            (300, 103),
            (400, 104),
            (510, 105),
            (620, 106),
            (730, 107),
            (840, 108),
            (950, 109),
            (951, 110),
        ];
        for (id, expected_res) in arr_id_and_res {
            let res = interior_binsearch!(payloads, id, |p: &InteriorTablePayload| p.key);

            assert_eq!(res, expected_res);
        }
    }
}

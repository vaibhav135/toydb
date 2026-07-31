mod child;
mod common;
mod root;

// Root represents cols
// Child represents rows
// Imagine them as nodes. Root at the top, everything else is a child node.

pub use child::{Child, ChildPayload, InteriorIndexPayload, LeafPayload};
pub use common::{BTreePageHeaderFormat, InteriorTablePayload};
pub use root::{DBFileInfo, DBHeader, Root, RootPage, RootPayload, SchemaType, SqlSchema};

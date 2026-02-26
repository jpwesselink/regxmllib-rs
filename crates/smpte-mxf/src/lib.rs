//! SMPTE ST 377-1 MXF file structure parsing.
//!
//! Provides [`PartitionPack`], [`PrimerPack`], [`RandomIndexPack`],
//! fill item detection, and partition seek helpers.

pub mod error;
pub mod fill;
pub mod partition;
pub mod primer;
pub mod rip;
pub mod seek;

pub use error::MxfError;
pub use fill::is_fill_item;
pub use partition::{is_partition_pack_key, PartitionKind, PartitionPack, PartitionStatus};
pub use primer::PrimerPack;
pub use rip::{RandomIndexPack, RipEntry};
pub use seek::{seek_footer_partition, seek_header_partition};

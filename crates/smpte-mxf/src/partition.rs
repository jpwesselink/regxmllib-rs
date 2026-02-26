use std::io::Cursor;

use smpte_klv::{
    ber::{read_u16_be, read_u32_be, read_u64_be},
    MemoryTriplet,
};
use smpte_types::Ul;

use crate::MxfError;

/// Partition Pack key template (SMPTE ST 377-1 §6.5).
/// Byte[12] = 0x01 (Partition Pack sub-type, always constant).
/// Byte[13] = kind (0x02 Header / 0x03 Body / 0x04 Footer) — skipped by MASK.
/// Byte[14] = status (0x01–0x04) — skipped by MASK.
const KEY: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x05, 0x01, 0x01,
    0x0D, 0x01, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00,
];

/// `equals_with_mask` mask for partition pack detection.
/// Compares bytes 0-6, 8-12, 15; ignores byte 7 (version), 13 (kind), 14 (status).
pub const PARTITION_PACK_MASK: u16 = 0xFEF9;

/// Broad scan mask used during run-in scanning.
/// Compares bytes 0-6, 8-10; ignores byte 7 and bytes 11-15.
pub const SCAN_MASK: u16 = 0xFEE0;

/// The role of an MXF partition (SMPTE ST 377-1 §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionKind {
    Header,
    Body,
    Footer,
}

/// Open/closed + complete/incomplete status bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionStatus {
    OpenIncomplete,
    ClosedIncomplete,
    OpenComplete,
    ClosedComplete,
}

/// Parsed MXF partition pack (SMPTE ST 377-1 §6.5).
#[derive(Debug, Clone)]
pub struct PartitionPack {
    pub kind: PartitionKind,
    pub status: PartitionStatus,
    pub major_version: u16,
    pub minor_version: u16,
    pub kag_size: u32,
    pub this_partition: u64,
    pub previous_partition: u64,
    pub footer_partition: u64,
    pub header_byte_count: u64,
    pub index_byte_count: u64,
    pub index_sid: u32,
    pub body_offset: u64,
    pub body_sid: u32,
    pub operational_pattern: Ul,
    pub essence_containers: Vec<Ul>,
}

impl PartitionPack {
    /// Return the template [`Ul`] used for partition pack detection.
    pub fn key() -> Ul {
        Ul::from_bytes(KEY)
    }

    /// Parse a partition pack from a [`MemoryTriplet`].
    ///
    /// Returns `None` if the triplet key does not match the partition pack key.
    pub fn from_triplet(triplet: &MemoryTriplet) -> Result<Option<Self>, MxfError> {
        let tpl = Ul::from_bytes(KEY);
        let ul = match triplet.key.as_ul() {
            Some(ul) => ul,
            None => return Ok(None),
        };
        if !ul.equals_with_mask(&tpl, PARTITION_PACK_MASK) {
            return Ok(None);
        }

        // Kind is at byte[13], status at byte[14] of the triplet key.
        let kind_byte = ul.as_bytes()[13];
        let status_byte = ul.as_bytes()[14];

        let kind = match kind_byte {
            0x02 => PartitionKind::Header,
            0x03 => PartitionKind::Body,
            0x04 => PartitionKind::Footer,
            _ => return Err(MxfError::InvalidPartitionKey),
        };

        let status = match status_byte {
            0x01 => PartitionStatus::OpenIncomplete,
            0x02 => PartitionStatus::ClosedIncomplete,
            0x03 => PartitionStatus::OpenComplete,
            0x04 => PartitionStatus::ClosedComplete,
            _ => return Err(MxfError::InvalidPartitionKey),
        };

        let mut c = Cursor::new(triplet.value.as_slice());

        let major_version    = read_u16_be(&mut c)?;
        let minor_version    = read_u16_be(&mut c)?;
        let kag_size         = read_u32_be(&mut c)?;
        let this_partition   = read_u64_be(&mut c)?;
        let previous_partition = read_u64_be(&mut c)?;
        let footer_partition = read_u64_be(&mut c)?;
        let header_byte_count = read_u64_be(&mut c)?;
        let index_byte_count  = read_u64_be(&mut c)?;
        let index_sid        = read_u32_be(&mut c)?;
        let body_offset      = read_u64_be(&mut c)?;
        let body_sid         = read_u32_be(&mut c)?;

        let mut op_bytes = [0u8; 16];
        std::io::Read::read_exact(&mut c, &mut op_bytes)?;
        let operational_pattern = Ul::from_bytes(op_bytes);

        // Essence containers — MXF batch: u32 count + u32 item_size + N×16 bytes
        let ec_count     = read_u32_be(&mut c)?;
        let _ec_item_len = read_u32_be(&mut c)?; // should be 16
        let mut essence_containers = Vec::with_capacity(ec_count as usize);
        for _ in 0..ec_count {
            let mut ul_bytes = [0u8; 16];
            std::io::Read::read_exact(&mut c, &mut ul_bytes)?;
            essence_containers.push(Ul::from_bytes(ul_bytes));
        }

        Ok(Some(PartitionPack {
            kind,
            status,
            major_version,
            minor_version,
            kag_size,
            this_partition,
            previous_partition,
            footer_partition,
            header_byte_count,
            index_byte_count,
            index_sid,
            body_offset,
            body_sid,
            operational_pattern,
            essence_containers,
        }))
    }
}

/// Returns `true` if `ul` looks like any partition pack key (broad scan).
pub fn is_partition_pack_key(ul: &Ul) -> bool {
    ul.equals_with_mask(&Ul::from_bytes(KEY), SCAN_MASK)
}

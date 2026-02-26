use std::io::{Read, Seek, SeekFrom};

use smpte_klv::KlvStream;
use smpte_types::Ul;

use crate::{
    partition::{is_partition_pack_key, PartitionPack},
    rip::RandomIndexPack,
    MxfError,
};

/// Maximum run-in length before the first partition pack (SMPTE ST 377-1 §6.4).
const MAX_RUN_IN: u64 = 65536;

/// Seek `r` to the start of the header partition pack key.
///
/// Implements the run-in scan from SMPTE ST 377-1 §6.4: reads 16-byte windows
/// sliding forward one byte at a time until a partition pack key prefix is found.
///
/// Returns the byte offset where the partition pack key starts.
pub fn seek_header_partition<R: Read + Seek>(r: &mut R) -> Result<u64, MxfError> {
    r.seek(SeekFrom::Start(0))?;

    let mut window = [0u8; 16];
    let mut offset: u64 = 0;

    while offset <= MAX_RUN_IN {
        r.seek(SeekFrom::Start(offset))?;
        match r.read(&mut window) {
            Ok(16) => {}
            Ok(_) | Err(_) => break,
        }
        let ul = Ul::from_bytes(window);
        if is_partition_pack_key(&ul) {
            r.seek(SeekFrom::Start(offset))?;
            return Ok(offset);
        }
        offset += 1;
    }

    Err(MxfError::PartitionNotFound)
}

/// Seek `r` to the start of the footer partition pack key.
///
/// Strategy:
/// 1. Parse the header partition pack and use its `footer_partition` field.
/// 2. If that is zero, read the last 4 bytes to locate a Random Index Pack,
///    then use the last entry's offset.
pub fn seek_footer_partition<R: Read + Seek>(r: &mut R) -> Result<u64, MxfError> {
    // --- Step 1: parse the header partition pack ---
    let header_offset = seek_header_partition(r)?;
    r.seek(SeekFrom::Start(header_offset))?;

    let mut stream = KlvStream::new(&mut *r);
    let triplet = stream.read_triplet()?.ok_or(MxfError::Truncated(
        "expected header partition pack triplet",
    ))?;

    let pp = PartitionPack::from_triplet(&triplet)?
        .ok_or(MxfError::Truncated("triplet is not a partition pack"))?;

    if pp.footer_partition != 0 {
        r.seek(SeekFrom::Start(pp.footer_partition))?;
        return Ok(pp.footer_partition);
    }

    // --- Step 2: fall back to RIP ---
    // The last 4 bytes of the file give the overall RIP KLV length
    // (from the start of the RIP key to the end of the RIP value).
    let file_len = r.seek(SeekFrom::End(0))?;
    if file_len < 4 {
        return Err(MxfError::Truncated("file too small to contain a RIP"));
    }

    r.seek(SeekFrom::End(-4))?;
    let mut rip_len_bytes = [0u8; 4];
    r.read_exact(&mut rip_len_bytes)?;
    let rip_overall_len = u32::from_be_bytes(rip_len_bytes) as u64;

    if rip_overall_len > file_len {
        return Err(MxfError::Truncated("RIP length exceeds file size"));
    }

    let rip_start = file_len - rip_overall_len;
    r.seek(SeekFrom::Start(rip_start))?;

    let mut stream = KlvStream::new(&mut *r);
    let triplet = stream
        .read_triplet()?
        .ok_or(MxfError::Truncated("expected RIP triplet at end of file"))?;

    let rip = RandomIndexPack::from_triplet(&triplet)?
        .ok_or(MxfError::Truncated("triplet at end of file is not a RIP"))?;

    let footer_offset = rip
        .entries
        .last()
        .ok_or(MxfError::Truncated("RIP has no entries"))?
        .byte_offset;

    r.seek(SeekFrom::Start(footer_offset))?;
    Ok(footer_offset)
}

use std::io::{Read, Seek, SeekFrom, Write};

use quick_xml::{
    events::{BytesDecl, Event},
    Writer,
};
use regxml_dict::DefinitionResolver;
use smpte_klv::{KlvStream, LocalSet};
use smpte_mxf::{is_fill_item, PartitionPack, PrimerPack};
use smpte_types::{Auid, Ul};

use crate::{
    event::EventHandler,
    fragment::{AuidNamer, FragmentBuilder},
    FragmentError,
};

// ── Key constants ─────────────────────────────────────────────────────────────

// Preface class key: 060e2b34.02530101.0d010101.01012f00
const PREFACE_KEY_BYTES: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x53, 0x01, 0x01, 0x0D, 0x01, 0x01, 0x01, 0x01, 0x01, 0x2F, 0x00,
];

// Version byte mask for UL comparison.
const PREFACE_KEY_MASK: u16 = 0xFF7F;

// Index Table Segment key: 060e2b34.02530101.0d010201.01100100
const INDEX_TABLE_KEY_BYTES: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x53, 0x01, 0x01, 0x0D, 0x01, 0x02, 0x01, 0x01, 0x10, 0x01, 0x00,
];

// InstanceID property: 060e2b34.01010101.01011502.00000000
const INSTANCE_UID_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x15, 0x02, 0x00, 0x00, 0x00, 0x00,
];

// GenericDescriptor class key: 060e2b34.02530101.0d010101.01012400
const GENERIC_DESCRIPTOR_KEY_BYTES: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x02, 0x53, 0x01, 0x01, 0x0D, 0x01, 0x01, 0x01, 0x01, 0x01, 0x24, 0x00,
];

// ── Public types ──────────────────────────────────────────────────────────────

/// Target partition for MXF header metadata extraction.
#[derive(Debug, Clone, Copy, Default)]
pub enum PartitionTarget {
    /// Try the footer partition first; fall back to header if unavailable or
    /// if the footer contains no primer pack (i.e. no metadata).
    #[default]
    Auto,
    /// Use the header partition only.
    Header,
    /// Use the footer partition only.
    Footer,
}

/// Which object to use as the root of the RegXML output.
#[derive(Debug, Clone, Copy, Default)]
pub enum RootMode {
    /// Emit the full Preface tree (default, equivalent to Java `-all`).
    #[default]
    Preface,
    /// Emit only the first EssenceDescriptor (equivalent to Java `-ed`).
    EssenceDescriptor,
}

/// Options for [`MxfFragmentBuilder`].
#[derive(Default)]
pub struct MxfFragmentOptions {
    pub partition: PartitionTarget,
    pub root_mode: RootMode,
    pub event_handler: Option<Box<dyn EventHandler>>,
    /// Optional UL → symbol resolver for ExtendibleEnumeration values.
    /// Populate from a SMPTE Labels register via `regxml_dict::LabelsRegister`.
    pub auid_namer: Option<Box<dyn AuidNamer>>,
}

/// Orchestrates a complete MXF → RegXML conversion pipeline.
pub struct MxfFragmentBuilder;

impl MxfFragmentBuilder {
    /// Read MXF from `reader`, write RegXML to `writer` using `dictionaries`.
    pub fn from_reader<R, W, D>(
        reader: &mut R,
        writer: W,
        dictionaries: &D,
        options: MxfFragmentOptions,
    ) -> Result<(), MxfFragmentError>
    where
        R: Read + Seek,
        W: Write,
        D: DefinitionResolver,
    {
        // Seek to the requested partition.
        seek_to_partition(reader, options.partition)?;

        // Extract all metadata sets from the partition.
        let (primer, all_sets) = extract_partition_metadata(reader)?;

        write_regxml(primer, all_sets, writer, dictionaries, options)
    }
}

// ── Pipeline stages ───────────────────────────────────────────────────────────

/// Seek `reader` to the start of the requested partition.
///
/// For `Auto`: tries footer first; if the footer seek fails OR the footer
/// partition has no PrimerPack (header_byte_count == 0), rewinds and uses
/// the header partition instead.
fn seek_to_partition<R: Read + Seek>(
    reader: &mut R,
    target: PartitionTarget,
) -> Result<(), MxfFragmentError> {
    match target {
        PartitionTarget::Header => {
            smpte_mxf::seek_header_partition(reader)?;
        }
        PartitionTarget::Footer => {
            smpte_mxf::seek_footer_partition(reader)?;
        }
        PartitionTarget::Auto => {
            let start = reader
                .stream_position()
                .map_err(|e| MxfFragmentError::Io(e.to_string()))?;

            let footer_ok =
                smpte_mxf::seek_footer_partition(reader).is_ok() && footer_has_metadata(reader);

            if !footer_ok {
                reader
                    .seek(SeekFrom::Start(start))
                    .map_err(|e| MxfFragmentError::Io(e.to_string()))?;
                smpte_mxf::seek_header_partition(reader)?;
            }
        }
    }
    Ok(())
}

/// Peek at the footer partition to determine if it contains header metadata
/// (header_byte_count > 0).  Leaves the reader positioned at the start of the
/// footer partition pack KLV triplet (i.e. it rewinds after the peek).
fn footer_has_metadata<R: Read + Seek>(reader: &mut R) -> bool {
    // Save position so we can restore it.
    let pos = match reader.stream_position() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let result = (|| -> Result<bool, MxfFragmentError> {
        let mut klv = KlvStream::new(&mut *reader);
        let triplet = klv
            .read_triplet()
            .map_err(|e| MxfFragmentError::Klv(e.to_string()))?
            .ok_or(MxfFragmentError::MissingPartitionPack)?;
        let pp =
            PartitionPack::from_triplet(&triplet)?.ok_or(MxfFragmentError::MissingPartitionPack)?;
        Ok(pp.header_byte_count > 0)
    })();

    // Restore position.
    let _ = reader.seek(SeekFrom::Start(pos));
    result.unwrap_or(false)
}

/// Read the partition pack, primer pack, and all local sets starting at the
/// current reader position (must be seeked to a partition already).
fn extract_partition_metadata<R: Read + Seek>(
    reader: &mut R,
) -> Result<(PrimerPack, Vec<LocalSet>), MxfFragmentError> {
    let mut klv = KlvStream::new(reader);

    // Read the partition pack.
    let pp_triplet = klv
        .read_triplet()
        .map_err(|e| MxfFragmentError::Klv(e.to_string()))?
        .ok_or(MxfFragmentError::MissingPartitionPack)?;
    let pp: PartitionPack =
        PartitionPack::from_triplet(&pp_triplet)?.ok_or(MxfFragmentError::MissingPartitionPack)?;

    let header_byte_count = pp.header_byte_count;

    // Find the PrimerPack.
    let primer = find_primer_pack(&mut klv)?;

    // Collect all local sets.
    let all_sets = collect_local_sets(&mut klv, &primer, header_byte_count)?;

    Ok((primer, all_sets))
}

/// Build the FragmentBuilder and write RegXML to `writer`.
fn write_regxml<W, D>(
    _primer: PrimerPack,
    all_sets: Vec<LocalSet>,
    writer: W,
    dictionaries: &D,
    options: MxfFragmentOptions,
) -> Result<(), MxfFragmentError>
where
    W: Write,
    D: DefinitionResolver,
{
    // Index sets by InstanceUID.
    let instance_uid_auid = Auid::from_bytes(INSTANCE_UID_UL);
    let mut sets_by_uid: std::collections::HashMap<Auid, LocalSet> =
        std::collections::HashMap::new();

    for set in &all_sets {
        if let Some(item) = set.items.iter().find(|i| i.key == instance_uid_auid) {
            if item.value.len() == 16 {
                let mut b = [0u8; 16];
                b.copy_from_slice(&item.value);
                sets_by_uid.insert(Auid::from_bytes(b), set.clone());
            }
        }
    }

    // Find the root set based on RootMode.
    let root = match options.root_mode {
        RootMode::Preface => {
            let preface_key = Ul::from_bytes(PREFACE_KEY_BYTES);
            all_sets
                .iter()
                .find(|s| {
                    s.key
                        .as_ul()
                        .map(|ul| ul.equals_with_mask(&preface_key, PREFACE_KEY_MASK))
                        .unwrap_or(false)
                })
                .ok_or(MxfFragmentError::MissingRootObject)?
        }
        RootMode::EssenceDescriptor => find_essence_descriptor(&all_sets, dictionaries)?,
    };

    let mut xml_writer = Writer::new_with_indent(writer, b' ', 2);

    xml_writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(|e| MxfFragmentError::Xml(e.to_string()))?;

    let mut builder = FragmentBuilder::new(dictionaries);
    if let Some(h) = options.event_handler {
        builder = builder.with_event_handler(h);
    }
    if let Some(n) = options.auid_namer {
        builder = builder.with_auid_namer(n);
    }
    for (uid, set) in sets_by_uid {
        builder.add_set(uid, set);
    }

    let root_clone = root.clone();
    builder.from_triplet(&root_clone, &mut xml_writer)?;

    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Recursively collect all AUIDs that are subclasses of `root_auid` (inclusive).
fn collect_descriptor_auids<D: DefinitionResolver>(
    resolver: &D,
    root_auid: Auid,
) -> std::collections::HashSet<Auid> {
    let mut result = std::collections::HashSet::new();
    let mut queue = vec![root_auid];
    while let Some(auid) = queue.pop() {
        if !result.insert(auid) {
            continue; // already visited
        }
        if let Some(regxml_dict::Definition::Class(cd)) = resolver.get_definition(&auid) {
            for sub in resolver.get_subclasses_of(cd) {
                queue.push(sub);
            }
        }
    }
    result
}

/// Known SMPTE byte[14] class codes for concrete descriptor types.
/// Derived from the SMPTE groups register.  Excludes Preface (0x2F),
/// ContentStorage (0x20), Identification (0x30), and other non-descriptor groups.
const DESCRIPTOR_CLASS_CODES: &[u8] = &[
    0x24, // GenericDescriptor
    0x25, // FileDescriptor
    0x27, // GenericPictureEssenceDescriptor
    0x28, // CDCIDescriptor
    0x29, // RGBADescriptor
    0x42, // SoundEssenceDescriptor
    0x43, // WaveAudioDescriptor / PCMDescriptor
    0x44, // AES3AudioDescriptor
    0x45, // BWFImportDescriptor
    0x48, // MultipleDescriptor
    0x49, // GenericDataEssenceDescriptor
    0x4A, // VBIDataDescriptor
    0x4B, // ANCDataDescriptor
    0x51, // DCTimedTextDescriptor / DC suites
    0x6B, // MXFGCGenericEssenceContainerDescriptor
];

/// Find the first EssenceDescriptor set in `all_sets`.
fn find_essence_descriptor<'a, D: DefinitionResolver>(
    all_sets: &'a [LocalSet],
    resolver: &D,
) -> Result<&'a LocalSet, MxfFragmentError> {
    // Collect all known descriptor class AUIDs by recursively walking the
    // subclass hierarchy rooted at GenericDescriptor.
    let generic_desc_auid = Auid::from_bytes(GENERIC_DESCRIPTOR_KEY_BYTES);
    let descriptor_auids = collect_descriptor_auids(resolver, generic_desc_auid);

    // Try dictionary-based lookup first (version-byte zeroed for comparison).
    if descriptor_auids.len() > 1 {
        for s in all_sets {
            if let Some(ul) = s.key.as_ul() {
                let exact = Auid::from_bytes(*ul.as_bytes());
                // Also try with version byte (byte 7) zeroed.
                let mut b = *ul.as_bytes();
                b[7] = 0;
                let masked = Auid::from_bytes(b);
                if descriptor_auids.contains(&exact) || descriptor_auids.contains(&masked) {
                    return Ok(s);
                }
            }
        }
    }

    // Fallback: whitelist of known SMPTE descriptor class codes (byte[14]).
    // Checks that the UL is in the standard AAF metadata group space
    // (bytes 8–13 = 0D.01.01.01.01.01) and byte[14] is a descriptor code.
    for s in all_sets {
        if let Some(ul) = s.key.as_ul() {
            let b = ul.as_bytes();
            if b[8] == 0x0D
                && b[9] == 0x01
                && b[10] == 0x01
                && b[11] == 0x01
                && b[12] == 0x01
                && b[13] == 0x01
                && DESCRIPTOR_CLASS_CODES.contains(&b[14])
            {
                return Ok(s);
            }
        }
    }

    Err(MxfFragmentError::MissingRootObject)
}

fn find_primer_pack<R: Read + Seek>(
    klv: &mut KlvStream<R>,
) -> Result<PrimerPack, MxfFragmentError> {
    for _ in 0..32 {
        let triplet = klv
            .read_triplet()
            .map_err(|e| MxfFragmentError::Klv(e.to_string()))?
            .ok_or(MxfFragmentError::MissingPrimerPack)?;
        if is_fill_item(&triplet) {
            continue;
        }
        if let Some(p) = PrimerPack::from_triplet(&triplet)? {
            return Ok(p);
        }
    }
    Err(MxfFragmentError::MissingPrimerPack)
}

fn collect_local_sets<R: Read + Seek>(
    klv: &mut KlvStream<R>,
    primer: &PrimerPack,
    header_byte_count: u64,
) -> Result<Vec<LocalSet>, MxfFragmentError> {
    let index_key = Ul::from_bytes(INDEX_TABLE_KEY_BYTES);
    let mut sets = Vec::new();
    let mut bytes_read: u64 = 0;

    loop {
        if header_byte_count > 0 && bytes_read >= header_byte_count {
            break;
        }

        let triplet = match klv
            .read_triplet()
            .map_err(|e| MxfFragmentError::Klv(e.to_string()))?
        {
            Some(t) => t,
            None => break,
        };

        // Stop at index table segment.
        if triplet
            .key
            .as_ul()
            .map(|ul| {
                *ul.as_bytes() == INDEX_TABLE_KEY_BYTES || ul.equals_with_mask(&index_key, 0xFF7F)
            })
            .unwrap_or(false)
        {
            break;
        }

        bytes_read += (16 + triplet.value.len() + 4) as u64;

        if is_fill_item(&triplet) {
            continue;
        }

        if smpte_mxf::is_partition_pack_key(
            triplet.key.as_ul().unwrap_or(&Ul::from_bytes([0u8; 16])),
        ) {
            continue;
        }

        match LocalSet::from_triplet(&triplet, &primer.local_tag_register) {
            Ok(set) => sets.push(set),
            Err(_) => continue,
        }
    }

    Ok(sets)
}

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum MxfFragmentError {
    #[error("MXF error: {0}")]
    Mxf(#[from] smpte_mxf::MxfError),
    #[error("fragment error: {0}")]
    Fragment(#[from] FragmentError),
    #[error("KLV error: {0}")]
    Klv(String),
    #[error("XML error: {0}")]
    Xml(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("no partition pack found")]
    MissingPartitionPack,
    #[error("no primer pack found")]
    MissingPrimerPack,
    #[error("no root object found in header metadata")]
    MissingRootObject,
}

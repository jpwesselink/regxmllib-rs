//! End-to-end smoke test: open a real MXF, walk the header metadata, and
//! resolve every group and property against the SMPTE metadictionaries.
//!
//! Nothing is asserted beyond "it doesn't panic" — this test is mainly meant
//! to be run with `cargo test -- --nocapture` to see the human-readable dump.

use std::{fs::File, io::BufReader, path::PathBuf};

use regxml_dict::{
    definition::{Definition, TypeDefinition},
    importer::import_registers,
    resolver::DefinitionResolver,
};
use smpte_klv::KlvStream;
use smpte_mxf::{is_fill_item, seek_header_partition, PrimerPack};
use smpte_types::Auid;

fn resources() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("resources")
}

fn read_bytes(path: &str) -> Vec<u8> {
    std::fs::read(resources().join(path)).expect("resource not found")
}

/// Hex-encode 16 bytes like `060e2b34.027f0101.…`
fn fmt_ul_bytes(b: &[u8]) -> String {
    assert!(b.len() >= 16);
    format!(
        "{:02x}{:02x}{:02x}{:02x}.{:02x}{:02x}{:02x}{:02x}.\
         {:02x}{:02x}{:02x}{:02x}.{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Render the raw bytes of a value as hex, capped at 32 bytes.
fn hex_preview(data: &[u8]) -> String {
    let cap = data.len().min(32);
    let hex: String = data[..cap]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    if data.len() > 32 {
        format!("{hex} … ({} bytes total)", data.len())
    } else {
        hex
    }
}

/// Interpret value bytes for well-known simple types and return a human
/// readable string.  Falls back to hex_preview for unknown/complex types.
fn interpret_value(type_def: &TypeDefinition, data: &[u8]) -> String {
    match type_def {
        TypeDefinition::Integer(td) => match (td.size, td.is_signed, data.len()) {
            (1, false, 1) => format!("{}", data[0]),
            (1, true, 1) => format!("{}", data[0] as i8),
            (2, false, 2) => format!("{}", u16::from_be_bytes([data[0], data[1]])),
            (2, true, 2) => format!("{}", i16::from_be_bytes([data[0], data[1]])),
            (4, false, 4) => format!("{}", u32::from_be_bytes(data[..4].try_into().unwrap())),
            (4, true, 4) => format!("{}", i32::from_be_bytes(data[..4].try_into().unwrap())),
            (8, false, 8) => format!("{}", u64::from_be_bytes(data[..8].try_into().unwrap())),
            (8, true, 8) => format!("{}", i64::from_be_bytes(data[..8].try_into().unwrap())),
            _ => hex_preview(data),
        },
        TypeDefinition::Rename(_) | TypeDefinition::StrongReference(_) => {
            if data.len() == 16 {
                format!("urn:smpte:ul:{}", fmt_ul_bytes(data))
            } else {
                hex_preview(data)
            }
        }
        TypeDefinition::String(_) => {
            // Try UTF-16 BE (MXF convention)
            if data.len() % 2 == 0 {
                let chars: Vec<u16> = data
                    .chunks_exact(2)
                    .map(|c| u16::from_be_bytes([c[0], c[1]]))
                    .collect();
                if let Ok(s) = String::from_utf16(&chars) {
                    let s = s.trim_end_matches('\0');
                    return format!("{s:?}");
                }
            }
            hex_preview(data)
        }
        _ => hex_preview(data),
    }
}

#[test]
fn walk_video1_header_metadata() {
    // ── 1. Load metadictionaries ─────────────────────────────────────────────
    let types_xml = read_bytes("registers/Types.xml");
    let elements_xml = read_bytes("registers/Elements.xml");
    let groups_xml = read_bytes("registers/Groups.xml");

    let dict = import_registers(&[&types_xml, &elements_xml, &groups_xml])
        .expect("import_registers failed");

    println!(
        "\n=== Dictionary loaded: {} definitions ===\n",
        dict.definition_count()
    );

    // ── 2. Open MXF and seek to header partition ─────────────────────────────
    let mxf_path = resources().join("test-mxf/video1.mxf");
    let file = File::open(&mxf_path).expect("MXF file not found");
    let mut reader = BufReader::new(file);

    seek_header_partition(&mut reader).expect("header partition not found");

    let mut stream = KlvStream::new(&mut reader);

    // Skip the partition pack itself
    let pp_triplet = stream
        .read_triplet()
        .unwrap()
        .expect("no triplet after seek");
    let pp = smpte_mxf::PartitionPack::from_triplet(&pp_triplet)
        .unwrap()
        .unwrap();

    println!(
        "Partition: {:?}  status={:?}  MXF {}.{}",
        pp.kind, pp.status, pp.major_version, pp.minor_version
    );
    println!(
        "  OP: urn:smpte:ul:{}",
        fmt_ul_bytes(pp.operational_pattern.as_bytes())
    );
    for ec in &pp.essence_containers {
        println!("  EC: urn:smpte:ul:{}", fmt_ul_bytes(ec.as_bytes()));
    }
    println!();

    // ── 3. Find the primer pack ──────────────────────────────────────────────
    let mut primer_opt = None;
    for _ in 0..20 {
        let t = match stream.read_triplet().unwrap() {
            Some(t) => t,
            None => break,
        };
        if is_fill_item(&t) {
            continue;
        }
        if let Some(p) = PrimerPack::from_triplet(&t).unwrap() {
            primer_opt = Some(p);
            break;
        }
    }
    let primer = primer_opt.expect("no primer pack found");
    println!(
        "Primer pack: {} local tags",
        primer.local_tag_register.len()
    );

    // ── 4. Read header metadata sets ─────────────────────────────────────────
    let tag_register = &primer.local_tag_register;
    let mut set_count = 0usize;
    let mut known_props = 0usize;
    let mut unknown_props = 0usize;

    while let Some(triplet) = stream.read_triplet().unwrap() {
        if is_fill_item(&triplet) {
            continue;
        }

        // Try to parse as a local set; skip non-local-set triplets.
        let local_set = match smpte_klv::LocalSet::from_triplet(&triplet, tag_register) {
            Ok(s) => s,
            Err(_) => continue,
        };

        set_count += 1;

        // Resolve the group's class definition from the dictionary
        let group_auid: Auid = local_set.key;
        let class_symbol = dict
            .get_definition(&group_auid)
            .and_then(|d| {
                if let Definition::Class(c) = d {
                    Some(c.symbol.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("?DARK?");

        println!("─── Set #{set_count}: {class_symbol}");

        // Walk each property item in the set
        for item in &local_set.items {
            let prop_auid: &Auid = &item.key;

            // Resolve property definition
            let (prop_name, type_desc, value_str) = match dict.get_definition(prop_auid) {
                Some(Definition::Property(pd)) => {
                    let type_name = dict
                        .get_definition(&pd.property_type)
                        .map(|d| d.symbol().to_owned())
                        .unwrap_or_else(|| "?".to_owned());

                    let val_str = dict
                        .get_definition(&pd.property_type)
                        .and_then(|d| {
                            if let Definition::Type(td) = d {
                                Some(interpret_value(td, &item.value))
                            } else {
                                None
                            }
                        })
                        .unwrap_or_else(|| hex_preview(&item.value));

                    known_props += 1;
                    (pd.symbol.clone(), type_name, val_str)
                }
                _ => {
                    unknown_props += 1;
                    let auid_str = format!("{prop_auid}");
                    (auid_str, "?".to_owned(), hex_preview(&item.value))
                }
            };

            println!("  {prop_name:<40}  [{type_desc:<30}]  {value_str}");
        }
        println!();
    }

    println!(
        "=== Totals: {set_count} sets, {known_props} known props, \
         {unknown_props} dark props ===\n"
    );

    assert!(set_count > 0, "no local sets parsed");
}

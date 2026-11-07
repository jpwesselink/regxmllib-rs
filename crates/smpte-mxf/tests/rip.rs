//! Random Index Pack parsing against real MXF bytes.

use std::{
    fs::File,
    io::{BufReader, Cursor},
    path::PathBuf,
};

use smpte_klv::KlvStream;
use smpte_mxf::{seek_footer_partition, PartitionKind, PartitionPack, RandomIndexPack};

fn path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// `tests/fixtures/video1-rip.bin` is the trailing RIP KLV (72 bytes) copied
/// verbatim from `resources/test-mxf/video1.mxf`.
#[test]
fn parses_rip_from_real_mxf_bytes() {
    let bytes = std::fs::read(path("tests/fixtures/video1-rip.bin")).unwrap();
    assert_eq!(
        u32::from_be_bytes(bytes[bytes.len() - 4..].try_into().unwrap()) as usize,
        bytes.len(),
        "fixture must be a complete RIP (overall length == fixture size)"
    );

    let triplet = KlvStream::new(Cursor::new(bytes))
        .read_triplet()
        .unwrap()
        .expect("RIP triplet");
    let rip = RandomIndexPack::from_triplet(&triplet)
        .unwrap()
        .expect("real RIP must be recognised");

    let entries: Vec<(u32, u64)> = rip
        .entries
        .iter()
        .map(|e| (e.body_sid, e.byte_offset))
        .collect();
    assert_eq!(entries, vec![(0, 0), (1, 0x335d), (0, 0x7684), (0, 0x77a6)]);
}

/// Every test MXF file ends with a RIP; each must parse and its last entry
/// must point at the footer partition.
#[test]
fn rip_last_entry_points_at_footer_in_all_test_files() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/test-mxf");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("mxf") {
            continue;
        }
        let data = std::fs::read(&p).unwrap();
        let n = u32::from_be_bytes(data[data.len() - 4..].try_into().unwrap()) as usize;
        let triplet = KlvStream::new(Cursor::new(&data[data.len() - n..]))
            .read_triplet()
            .unwrap()
            .unwrap();
        let rip = RandomIndexPack::from_triplet(&triplet)
            .unwrap()
            .unwrap_or_else(|| panic!("{}: RIP not recognised", p.display()));
        let footer = rip.entries.last().unwrap().byte_offset as usize;

        let footer_triplet = KlvStream::new(Cursor::new(&data[footer..]))
            .read_triplet()
            .unwrap()
            .unwrap();
        let pp = PartitionPack::from_triplet(&footer_triplet)
            .unwrap()
            .unwrap_or_else(|| panic!("{}: RIP last entry is not a partition", p.display()));
        assert_eq!(pp.kind, PartitionKind::Footer, "{}", p.display());

        // seek_footer_partition must agree.
        let mut r = BufReader::new(File::open(&p).unwrap());
        assert_eq!(seek_footer_partition(&mut r).unwrap() as usize, footer);
        checked += 1;
    }
    assert!(
        checked >= 9,
        "expected the test-mxf corpus, found {checked}"
    );
}

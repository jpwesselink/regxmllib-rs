use std::{fs::File, io::BufReader, path::PathBuf};

use smpte_klv::KlvStream;
use smpte_mxf::{
    seek_header_partition, seek_footer_partition,
    PartitionKind, PartitionPack, PrimerPack,
};

fn test_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("resources/test-mxf")
        .join(name)
}

fn open(name: &str) -> BufReader<File> {
    BufReader::new(File::open(test_file(name)).expect("test MXF file not found"))
}

/// Parse the header partition pack from a file and return it.
fn read_header_pp(name: &str) -> PartitionPack {
    let mut r = open(name);
    let offset = seek_header_partition(&mut r).expect("header partition not found");
    assert!(offset <= 65536, "offset beyond run-in limit");

    let mut stream = KlvStream::new(&mut r);
    let triplet = stream.read_triplet().unwrap().expect("no triplet after seek");
    PartitionPack::from_triplet(&triplet)
        .expect("from_triplet error")
        .expect("triplet is not a partition pack")
}

#[test]
fn header_partition_video1() {
    let pp = read_header_pp("video1.mxf");
    assert_eq!(pp.kind, PartitionKind::Header);
    assert_eq!(pp.major_version, 1);
}

#[test]
fn header_partition_audio1() {
    let pp = read_header_pp("audio1.mxf");
    assert_eq!(pp.kind, PartitionKind::Header);
    assert_eq!(pp.major_version, 1);
}

#[test]
fn header_partition_class14() {
    let pp = read_header_pp("class14.mxf");
    assert_eq!(pp.kind, PartitionKind::Header);
}

#[test]
fn primer_pack_present_video1() {
    let mut r = open("video1.mxf");
    seek_header_partition(&mut r).expect("header partition not found");

    let mut stream = KlvStream::new(&mut r);
    // Skip the partition pack triplet
    stream.read_triplet().unwrap().unwrap();

    // Next non-fill triplet should be the primer pack
    let mut primer_found = false;
    for _ in 0..10 {
        let t = match stream.read_triplet().unwrap() {
            Some(t) => t,
            None => break,
        };
        if smpte_mxf::is_fill_item(&t) {
            continue;
        }
        if PrimerPack::from_triplet(&t).unwrap().is_some() {
            primer_found = true;
            break;
        }
    }
    assert!(primer_found, "no primer pack found after header partition pack");
}

#[test]
fn open_incomplete_header_parses() {
    // This file has an open-incomplete header partition — edge case
    let pp = read_header_pp("open-incomplete-header.mxf");
    assert_eq!(pp.kind, PartitionKind::Header);
    assert_eq!(pp.status, smpte_mxf::PartitionStatus::OpenIncomplete);
}

#[test]
fn footer_partition_reachable() {
    let mut r = open("video1.mxf");
    // footer_partition == 0 means we fall back to RIP; either way it must succeed
    let footer_offset = seek_footer_partition(&mut r).expect("footer partition not found");

    let mut stream = KlvStream::new(&mut r);
    let triplet = stream.read_triplet().unwrap().expect("no triplet at footer");
    let pp = PartitionPack::from_triplet(&triplet)
        .expect("from_triplet error")
        .expect("footer is not a partition pack");

    assert_eq!(pp.kind, PartitionKind::Footer, "expected footer at offset {footer_offset}");
}

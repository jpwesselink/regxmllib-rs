//! Golden-file integration tests: MXF → RegXML via MxfFragmentBuilder.
//! Each test converts an MXF file and compares the output leaf-by-leaf against
//! the Java reference RegXML output.
//!
//! Comparison is semantic (Clark notation `{ns}localname`, trimmed text) so
//! namespace prefix differences (ns1/ns2 vs r0/r1) do not cause false failures.

use std::{fs::File, io::BufReader, path::PathBuf};

use regxml::{MxfFragmentBuilder, MxfFragmentOptions};
use regxml_dict::importer::import_registers;

// ── XML semantic comparison helpers ─────────────────────────────────────────

/// A flattened view of an XML document: a sequence of (clark_tag, trimmed_text) pairs.
/// Only leaf text nodes are included.
#[derive(Debug, PartialEq)]
struct XmlLeaf {
    tag: String,
    text: String,
}

/// Parse XML bytes into a flat list of leaves, discarding attribute order and
/// namespace prefixes so the comparison is purely structural.
fn flatten_xml(xml: &[u8]) -> Vec<XmlLeaf> {
    use quick_xml::{events::Event, name::ResolveResult, NsReader};

    let mut reader = NsReader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut leaves = Vec::new();
    let mut tag_stack: Vec<String> = Vec::new();

    loop {
        match reader.read_resolved_event() {
            Ok((ns, Event::Start(e))) => {
                let ns_str = match ns {
                    ResolveResult::Bound(n) => {
                        std::str::from_utf8(n.into_inner()).unwrap_or("").to_owned()
                    }
                    _ => String::new(),
                };
                let local = std::str::from_utf8(e.local_name().into_inner())
                    .unwrap_or("")
                    .to_owned();
                let clark = if ns_str.is_empty() {
                    local
                } else {
                    format!("{{{ns_str}}}{local}")
                };
                tag_stack.push(clark);
            }
            Ok((_, Event::Text(t))) => {
                // Decode XML entities (&amp; &lt; &quot; &#NNN; etc.) before
                // comparing so that different serialisation choices (e.g. `"`
                // vs `&quot;`) do not cause false mismatches.
                let text = t
                    .unescape()
                    .map(|s| s.trim().to_owned())
                    .unwrap_or_default();
                if !text.is_empty() {
                    if let Some(tag) = tag_stack.last().cloned() {
                        leaves.push(XmlLeaf { tag, text });
                    }
                }
            }
            Ok((_, Event::End(_))) => {
                tag_stack.pop();
            }
            Ok((_, Event::Eof)) => break,
            Ok(_) => {}
            Err(e) => panic!("XML parse error: {e}"),
        }
    }
    leaves
}

// ── Shared test infrastructure ───────────────────────────────────────────────

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

/// Convert `mxf_name.mxf` to RegXML and compare against `mxf_name.xml` golden.
fn run_golden(mxf_name: &str) {
    let types_xml = read_bytes("registers/Types.xml");
    let elements_xml = read_bytes("registers/Elements.xml");
    let groups_xml = read_bytes("registers/Groups.xml");

    let dict = import_registers(&[&types_xml, &elements_xml, &groups_xml])
        .expect("import_registers failed");

    let mxf_path = resources().join(format!("test-mxf/{mxf_name}.mxf"));
    let file = File::open(&mxf_path).unwrap_or_else(|_| panic!("MXF file not found: {mxf_path:?}"));
    let mut reader = BufReader::new(file);

    let mut xml_output = Vec::new();
    MxfFragmentBuilder::from_reader(
        &mut reader,
        &mut xml_output,
        &dict,
        MxfFragmentOptions::default(),
    )
    .unwrap_or_else(|e| panic!("MxfFragmentBuilder failed for {mxf_name}: {e}"));

    let golden_bytes = read_bytes(&format!("test-regxml/{mxf_name}.xml"));

    let ours = flatten_xml(&xml_output);
    let golden = flatten_xml(&golden_bytes);

    let len = ours.len().max(golden.len());
    let mut mismatches = Vec::new();
    for i in 0..len {
        match (ours.get(i), golden.get(i)) {
            (Some(o), Some(g)) if o == g => {}
            (Some(o), Some(g)) => {
                mismatches.push(format!(
                    "leaf {i}:\n  golden: {} = '{}'\n  ours:   {} = '{}'",
                    g.tag, g.text, o.tag, o.text
                ));
            }
            (None, Some(g)) => {
                mismatches.push(format!(
                    "leaf {i}: golden has extra: {} = '{}'",
                    g.tag, g.text
                ));
            }
            (Some(o), None) => {
                mismatches.push(format!(
                    "leaf {i}: ours has extra: {} = '{}'",
                    o.tag, o.text
                ));
            }
            (None, None) => unreachable!(),
        }
    }

    if !mismatches.is_empty() {
        panic!(
            "{mxf_name}: {} leaf mismatch(es) vs golden:\n{}",
            mismatches.len(),
            mismatches.join("\n")
        );
    }
}

// ── Golden-file tests ────────────────────────────────────────────────────────

#[test]
fn golden_audio1() {
    run_golden("audio1");
}

#[test]
fn golden_audio2() {
    run_golden("audio2");
}

#[test]
fn golden_class14() {
    run_golden("class14");
}

#[test]
fn golden_escape_chars() {
    run_golden("escape-chars");
}

#[test]
fn golden_indirect() {
    run_golden("indirect");
}

#[test]
fn golden_utf8_embedded_text() {
    run_golden("utf8_embedded_text");
}

#[test]
fn golden_video1() {
    run_golden("video1");
}

#[test]
fn golden_video2() {
    run_golden("video2");
}

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use regxml::{
    AuidNamer, MxfFragmentBuilder, MxfFragmentOptions, PartitionTarget, RootMode, XmlSchemaBuilder,
};
use regxml_dict::{importer::import_registers, LabelsRegister, MetaDictionary};
use smpte_types::Auid;

// ── Embedded SMPTE registers ──────────────────────────────────────────────────

const EMBEDDED_ELEMENTS: &[u8] = include_bytes!("../registers/Elements.xml");
const EMBEDDED_GROUPS:   &[u8] = include_bytes!("../registers/Groups.xml");
const EMBEDDED_TYPES:    &[u8] = include_bytes!("../registers/Types.xml");
const EMBEDDED_LABELS:   &[u8] = include_bytes!("../registers/Labels.xml");

// ── LabelsNamer ───────────────────────────────────────────────────────────────

/// Wraps a [`LabelsRegister`] as an [`AuidNamer`] for use in
/// [`MxfFragmentOptions::auid_namer`].
struct LabelsNamer(LabelsRegister);

impl AuidNamer for LabelsNamer {
    fn name_of(&self, id: &Auid) -> Option<&str> {
        self.0.get(id)
    }
}

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Resolve an output path: if `path` is an existing directory, append
/// `default_filename`; otherwise use `path` as-is.
pub fn resolve_output(path: &Path, default_filename: &str) -> PathBuf {
    if path.is_dir() {
        path.join(default_filename)
    } else {
        path.to_path_buf()
    }
}

/// Expand a list of paths: directories are replaced by their contained
/// `*.xml` files (sorted); regular files are kept as-is.
pub fn expand_dict_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
                .with_context(|| format!("reading directory {path:?}"))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("xml"))
                .collect();
            entries.sort();
            result.extend(entries);
        } else {
            result.push(path.clone());
        }
    }
    Ok(result)
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn run_regxml_dump(
    dict_paths: &[PathBuf],
    label_paths: &[PathBuf],
    input: &Path,
    output: Option<&Path>,
    partition: PartitionTarget,
    root_mode: RootMode,
) -> Result<()> {
    // Load dictionary: use embedded SMPTE registers if no -d provided.
    let dict = if dict_paths.is_empty() {
        tracing::debug!("using embedded SMPTE registers");
        import_registers(&[EMBEDDED_ELEMENTS, EMBEDDED_GROUPS, EMBEDDED_TYPES])
            .context("loading embedded SMPTE registers")?
    } else {
        let paths = expand_dict_paths(dict_paths)?;
        let xml_bytes: Vec<Vec<u8>> = paths
            .iter()
            .map(|path| std::fs::read(path).with_context(|| format!("reading {path:?}")))
            .collect::<Result<_>>()?;
        let slices: Vec<&[u8]> = xml_bytes.iter().map(|bytes| bytes.as_slice()).collect();
        import_registers(&slices).context("importing SMPTE register XMLs")?
    };

    tracing::debug!(defs = dict.definition_count(), "dictionary loaded");

    // Load labels: use embedded Labels.xml if no -l provided.
    let auid_namer: Option<Box<dyn AuidNamer>> = if label_paths.is_empty() {
        tracing::debug!("using embedded SMPTE labels");
        let reg = LabelsRegister::from_xml(EMBEDDED_LABELS)
            .context("loading embedded SMPTE labels")?;
        Some(Box::new(LabelsNamer(reg)))
    } else {
        let label_paths = expand_dict_paths(label_paths)?;
        let mut merged = LabelsRegister::empty();
        for path in &label_paths {
            let xml = std::fs::read(path).with_context(|| format!("reading labels {path:?}"))?;
            let reg = LabelsRegister::from_xml(&xml)
                .with_context(|| format!("parsing labels register {path:?}"))?;
            merged = merged.merge(reg);
        }
        tracing::debug!(entries = merged.len(), "labels register loaded");
        Some(Box::new(LabelsNamer(merged)))
    };

    let file = File::open(input).with_context(|| format!("opening MXF file {input:?}"))?;
    let mut reader = BufReader::new(file);

    let options = MxfFragmentOptions {
        partition,
        root_mode,
        event_handler: None,
        auid_namer,
    };

    match output {
        Some(path) => {
            let out =
                File::create(path).with_context(|| format!("creating output file {path:?}"))?;
            let mut writer = BufWriter::new(out);
            MxfFragmentBuilder::from_reader(&mut reader, &mut writer, &dict, options)
                .context("MXF to RegXML conversion")?;
            writer.flush().context("flushing output")?;
        }
        None => {
            let stdout = std::io::stdout();
            let mut writer = BufWriter::new(stdout.lock());
            MxfFragmentBuilder::from_reader(&mut reader, &mut writer, &dict, options)
                .context("MXF to RegXML conversion")?;
            writer.flush().context("flushing output")?;
        }
    }

    Ok(())
}

/// Convert SMPTE register XMLs to a metadictionary.
///
/// `output` may be a directory (writes `metadict.xml` inside it, Java compat)
/// or a file path.
pub fn run_xml_registers_to_dict(inputs: &[PathBuf], output: &Path) -> Result<()> {
    let output = resolve_output(output, "metadict.xml");
    let xml_bytes: Vec<Vec<u8>> = inputs
        .iter()
        .map(|path| std::fs::read(path).with_context(|| format!("reading {path:?}")))
        .collect::<Result<_>>()?;
    let slices: Vec<&[u8]> = xml_bytes.iter().map(|bytes| bytes.as_slice()).collect();

    let dict = import_registers(&slices).context("importing SMPTE register XMLs")?;
    eprintln!(
        "Imported {} definitions from {} register file(s)",
        dict.definition_count(),
        inputs.len()
    );

    let xml_out = dict.to_xml().context("serializing metadictionary to XML")?;

    let mut out = File::create(&output).with_context(|| format!("creating {output:?}"))?;
    out.write_all(&xml_out)
        .with_context(|| format!("writing {output:?}"))?;

    eprintln!(
        "Wrote metadictionary ({} bytes) to {:?}",
        xml_out.len(),
        output
    );

    Ok(())
}

/// Generate an XSD schema from compiled metadictionary files, or from the
/// embedded SMPTE registers if no `-d` is provided.
///
/// `output` may be a directory (writes `schema.xsd` inside it, Java compat)
/// or a file path.
pub fn run_gen_dict_xsd(dict_paths: &[PathBuf], output: &Path) -> Result<()> {
    let output = resolve_output(output, "schema.xsd");

    let merged = if dict_paths.is_empty() {
        tracing::debug!("using embedded SMPTE registers for XSD generation");
        import_registers(&[EMBEDDED_ELEMENTS, EMBEDDED_GROUPS, EMBEDDED_TYPES])
            .context("loading embedded SMPTE registers")?
    } else {
        let mut merged = MetaDictionary::new("", "");
        for path in dict_paths {
            let xml = std::fs::read(path).with_context(|| format!("reading {path:?}"))?;
            let dict = MetaDictionary::from_xml(&xml)
                .with_context(|| format!("parsing metadictionary {path:?}"))?;
            for definition in dict.all_definitions() {
                merged.add(definition.clone()).ok();
            }
        }
        eprintln!(
            "Loaded {} definitions from {} metadictionary file(s)",
            merged.definition_count(),
            dict_paths.len()
        );
        merged
    };

    let out = File::create(&output).with_context(|| format!("creating {output:?}"))?;
    let mut writer = BufWriter::new(out);

    let builder = XmlSchemaBuilder::new(&merged);
    builder
        .from_dictionary(&merged, &mut writer)
        .context("generating XSD")?;
    writer.flush().context("flushing output")?;

    eprintln!("Wrote XSD to {:?}", output);

    Ok(())
}

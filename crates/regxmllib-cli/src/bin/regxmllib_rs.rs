use anyhow::{Context, Result};
use clap::{ArgGroup, Parser, Subcommand};
use std::path::PathBuf;

use regxml::{PartitionTarget, RootMode};
use regxmllib_cli::{run_gen_dict_xsd, run_regxml_dump, run_xml_registers_to_dict};

// ── Argv normalization ────────────────────────────────────────────────────────

/// Convert Java-style single-dash long options to double-dash so clap can
/// parse them.  Single-char flags (-i, -d, -e, ...) are left unchanged.
///
/// Examples: -all → --all, -header → --header, -auto → --auto
fn normalize_args() -> Vec<String> {
    std::env::args()
        .map(|arg| {
            if arg.starts_with('-')
                && !arg.starts_with("--")
                && arg.len() > 2
                && arg[1..].chars().all(|c| c.is_ascii_alphabetic())
            {
                format!("-{arg}")
            } else {
                arg
            }
        })
        .collect()
}

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "regxmllib-rs",
    about = "Java-free RegXML toolchain — drop-in replacement for sandflow/regxmllib",
    long_about = "Prepend `regxmllib-rs` to any existing Java regxmllib invocation:\n\n  \
        java -cp regxmllib.jar RegXMLDump -all -header -d dicts/ -i file.mxf\n  \
        regxmllib-rs RegXMLDump -all -header -d dicts/ -i file.mxf"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    // ── RegXMLDump ────────────────────────────────────────────────────────────
    #[command(
        name = "RegXMLDump",
        visible_aliases = ["regxml-dump", "regxml_dump", "dump"],
        about = "Export MXF header metadata as RegXML (SMPTE ST 2001-1)",
        group(ArgGroup::new("mode").required(true).args(["all", "ed"])),
        group(ArgGroup::new("partition").required(true).args(["header", "footer", "auto"]))
    )]
    RegxmlDump {
        /// Dictionary directory or register XML files (Elements.xml, Groups.xml, Types.xml)
        #[arg(short = 'd', long, num_args = 1.., value_name = "PATH", required = true)]
        dict: Vec<PathBuf>,

        /// Labels register XML file(s) for ExtendibleEnumeration symbol resolution
        #[arg(short = 'l', long = "labels", num_args = 1.., value_name = "PATH")]
        labels: Vec<PathBuf>,

        /// Input MXF file
        #[arg(short = 'i', long, value_name = "FILE", required = true)]
        input: PathBuf,

        /// Output file (stdout if omitted)
        #[arg(short = 'o', long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Use the header partition
        #[arg(long = "header", conflicts_with_all = ["footer", "auto"])]
        header: bool,

        /// Use the footer partition
        #[arg(long = "footer", conflicts_with_all = ["header", "auto"])]
        footer: bool,

        /// Use footer if available, fall back to header
        #[arg(long = "auto", conflicts_with_all = ["header", "footer"])]
        auto: bool,

        /// Dump all header metadata (full Preface tree)
        #[arg(long = "all", conflicts_with = "ed")]
        all: bool,

        /// Dump only the first EssenceDescriptor
        #[arg(long = "ed", conflicts_with = "all")]
        ed: bool,
    },

    // ── XMLRegistersToDict ────────────────────────────────────────────────────
    #[command(
        name = "XMLRegistersToDict",
        visible_aliases = ["xml-registers-to-dict", "xml_registers_to_dict"],
        about = "Convert SMPTE register XMLs into a compiled metadictionary XML"
    )]
    XmlRegistersToDict {
        /// Elements register XML file
        #[arg(short = 'e', long = "elements", value_name = "FILE")]
        elements: Option<PathBuf>,

        /// Groups register XML file
        #[arg(short = 'g', long = "groups", value_name = "FILE")]
        groups: Option<PathBuf>,

        /// Types register XML file
        #[arg(short = 't', long = "types", value_name = "FILE")]
        types: Option<PathBuf>,

        /// Labels register XML file
        #[arg(short = 'l', long = "labels", value_name = "FILE")]
        labels: Option<PathBuf>,

        /// Output directory or file (directory → writes metadict.xml inside it)
        #[arg(value_name = "OUTPUT", required = true)]
        output: PathBuf,
    },

    // ── GenerateDictionaryXMLSchema ───────────────────────────────────────────
    #[command(
        name = "GenerateDictionaryXMLSchema",
        visible_aliases = ["gen-dict-xsd", "gen_dict_xsd"],
        about = "Generate an XSD schema from a compiled RegXML metadictionary"
    )]
    GenDictXsd {
        /// Compiled metadictionary XML file(s) (output of XMLRegistersToDict)
        #[arg(short = 'd', long, num_args = 1.., value_name = "PATH", required = true)]
        dict: Vec<PathBuf>,

        /// Output directory or file (directory → writes schema.xsd inside it)
        #[arg(short = 'o', long, value_name = "PATH", required = true)]
        output: PathBuf,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse_from(normalize_args());

    match cli.command {
        Command::RegxmlDump {
            dict,
            labels,
            input,
            output,
            header,
            footer,
            auto: _,
            all: _,
            ed,
        } => {
            let partition = if footer {
                PartitionTarget::Footer
            } else if header {
                PartitionTarget::Header
            } else {
                PartitionTarget::Auto
            };
            let root_mode = if ed {
                RootMode::EssenceDescriptor
            } else {
                RootMode::Preface
            };
            run_regxml_dump(
                &dict,
                &labels,
                &input,
                output.as_deref(),
                partition,
                root_mode,
            )
            .context("RegXMLDump")
        }

        Command::XmlRegistersToDict {
            elements,
            groups,
            types,
            labels,
            output,
        } => {
            let mut inputs: Vec<PathBuf> = Vec::new();
            if let Some(p) = elements {
                inputs.push(p);
            }
            if let Some(p) = groups {
                inputs.push(p);
            }
            if let Some(p) = types {
                inputs.push(p);
            }
            if let Some(p) = labels {
                inputs.push(p);
            }
            if inputs.is_empty() {
                anyhow::bail!("no register XML files provided; use -e, -g, -t, -l");
            }
            run_xml_registers_to_dict(&inputs, &output).context("XMLRegistersToDict")
        }

        Command::GenDictXsd { dict, output } => {
            run_gen_dict_xsd(&dict, &output).context("GenerateDictionaryXMLSchema")
        }
    }
}

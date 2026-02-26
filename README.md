# regxmllib-rs

A native Rust port of [sandflow/regxmllib](https://github.com/sandflow/regxmllib) — the reference implementation of SMPTE ST 2001-1 RegXML.

Converts MXF (Material Exchange Format) header metadata into RegXML, an XML representation defined by SMPTE ST 2001-1. Output is semantically identical to the Java reference across all tested MXF files.

## Why

- **No JVM required.** Runs anywhere Rust runs — statically linked, no runtime dependencies.
- **Embeddable.** Each layer is a standalone library crate usable independently.
- **npm-packageable.** Prebuilt binaries can be distributed via npm for use in JS/TS toolchains (`npx regxmllib-rs dump ...`).

## Scope

| Standard | Coverage |
|---|---|
| SMPTE ST 2001-1 | RegXML fragment builder, XSD schema generator |
| SMPTE ST 377-1 | MXF partition packs, primer packs, RIP, header/footer seeking |
| SMPTE ST 336 | KLV triplets, BER length decoding, local sets |
| SMPTE ST 335 / ST 395 | Register XML import, metadictionary, definition resolution |

## Crates

```
smpte-types      UL, AUID, UUID, UMID, HalfFloat
smpte-klv        KLV stream, BER decoder, local sets
smpte-mxf        Partition/primer/RIP parsing, file seeking
regxml-dict      SMPTE register XML import, metadictionary, definition resolver
regxml           FragmentBuilder (ST 2001-1 Rules 3–5), MxfFragmentBuilder, XmlSchemaBuilder
regxmllib-cli    regxml-dump, xml-registers-to-dict, gen-dict-xsd
```

Dependency order: `smpte-types` → `smpte-klv` → `smpte-mxf` → `regxml-dict` → `regxml` → `regxmllib-cli`

## Drop-in replacement for Java regxmllib

Existing Java invocations work unchanged — just prepend `regxmllib-rs` (or `npx regxmllib-rs`):

```bash
# Java
java -cp regxmllib.jar RegXMLDump -all -header -d dicts/ -i file.mxf

# Rust — identical flags, no JVM
regxmllib-rs RegXMLDump -all -header -d dicts/ -i file.mxf
npx regxmllib-rs RegXMLDump -all -header -d dicts/ -i file.mxf
```

| Task | Java (`sandflow/regxmllib`) | Rust (`regxmllib-rs`) |
|---|---|---|
| Dump MXF as RegXML | `RegXMLDump -all -header -d <dicts/> -i file.mxf` | `regxmllib-rs RegXMLDump -all -header -d <dicts/> -i file.mxf` |
| Dump only EssenceDescriptor | `RegXMLDump -ed -auto -d <dicts/> -i file.mxf` | `regxmllib-rs RegXMLDump -ed -auto -d <dicts/> -i file.mxf` |
| Convert registers to metadictionary | `XMLRegistersToDict -e Elements.xml -l Labels.xml -g Groups.xml -t Types.xml <outdir/>` | `regxmllib-rs XMLRegistersToDict -e Elements.xml -l Labels.xml -g Groups.xml -t Types.xml <outdir/>` |
| Generate XSD from metadictionary | `GenerateDictionaryXMLSchema -d dict1 dict2 -o <outdir/>` | `regxmllib-rs GenerateDictionaryXMLSchema -d dict1 dict2 -o <outdir/>` |

Notable differences:

- **No pre-compilation step.** `RegXMLDump` reads raw SMPTE register XMLs directly. The Java tool requires pre-compiled metadictionary files from `XMLRegistersToDict` first.
- **Single output file.** `XMLRegistersToDict` writes one combined `metadict.xml` instead of one file per namespace. `GenerateDictionaryXMLSchema` writes one `schema.xsd`.
- **`-d` accepts directories.** Pass a directory and all `*.xml` files inside are used automatically.

## CLI

All commands are subcommands of the single `regxmllib-rs` binary.

### RegXMLDump — Export MXF header metadata as RegXML

```bash
regxmllib-rs RegXMLDump \
  -all -header \
  -d resources/registers/ \
  -i video.mxf
```

Options:
- `-d` — register XML directory or individual files (Elements.xml, Groups.xml, Types.xml)
- `-l` — SMPTE Labels register XML for ExtendibleEnumeration symbol resolution
- `-i` — input MXF file
- `-o` — output file (default: stdout)
- `-header` / `-footer` / `-auto` — partition selection (required)
- `-all` / `-ed` — full Preface tree or EssenceDescriptor only (required)

### XMLRegistersToDict — Convert SMPTE registers to metadictionary

```bash
regxmllib-rs XMLRegistersToDict \
  -e resources/registers/Elements.xml \
  -g resources/registers/Groups.xml \
  -t resources/registers/Types.xml \
  -l resources/registers/Labels.xml \
  outdir/
```

Writes `metadict.xml` into the output directory.

### GenerateDictionaryXMLSchema — Generate XSD from metadictionary

```bash
regxmllib-rs GenerateDictionaryXMLSchema \
  -d metadict.xml \
  -o outdir/
```

Writes `schema.xsd` into the output directory.

## Development

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

### Golden-file tests

The integration test suite (`crates/regxmllib-cli/tests/mxf_to_regxml.rs`) converts 8 MXF files and compares the output leaf-by-leaf against Java reference RegXML. Comparison is semantic — namespace prefix differences do not cause failures.

```bash
cargo test -p regxmllib-cli --test mxf_to_regxml
```

## Verification against Java reference

The Rust output has been verified against the original Java `RegXMLDump` tool on **75 real-world MXF files** from IMF packages including:

- DCI/IMF video (JPEG 2000, PHDR)
- Multi-channel audio (PCM, Dolby Atmos, IAB)
- Timed text (IMSC, ISXD)
- SADM / ST 2067-203 audio
- Netflix Photon and AMWA plugfest test content

All files produced semantically identical output.

## Resources

```
resources/
  registers/        SMPTE Elements, Groups, Types, Labels register XMLs
  regxml-dicts/     Precompiled metadictionary XMLs (three baseline namespaces)
  test-mxf/         9 MXF files used by the test suite
  test-regxml/      Java reference RegXML golden files (8 files)
```

## Releasing to npm

### Via GitHub Actions (all platforms)

1. Bump the version across all packages:
   ```bash
   cd npm && pnpm -r version 0.2.0
   ```

2. Commit and push the version bump.

3. Go to **Actions → Publish npm packages → Run workflow**.
   - Leave **dry_run** checked to pack without publishing.
   - Uncheck **dry_run** to publish to npmjs.com (requires `NPM_TOKEN` secret).

The workflow builds binaries for all 6 platforms in parallel, stages them, and publishes each platform package followed by the launcher.

### Locally (current platform only)

Requires Node ≥ 22.12 and pnpm.

```bash
cd npm

# dry run — build + pack, nothing published
node ./scripts/local-publish.ts

# publish current platform package + launcher
node ./scripts/local-publish.ts --publish
```

Scripts are `.ts` — Node ≥ 22.12 runs them directly without any transpilation step.

This builds the Rust binary for the host platform, stages it into the correct package, and publishes only that platform package plus the `regxmllib-rs` launcher.

> When publishing locally you only release the binary for your current machine. Use the GitHub Actions workflow to release all platforms simultaneously.

## Related

- [sandflow/regxmllib](https://github.com/sandflow/regxmllib) — original Java reference implementation
- [SMPTE ST 2001-1](https://www.smpte.org/) — RegXML specification
- [SMPTE ST 377-1](https://www.smpte.org/) — MXF specification

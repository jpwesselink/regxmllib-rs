# regxmllib-rs (npm)

Node launcher package for prebuilt `regxmllib` Rust binaries.

Installs the correct native binary for your platform automatically.

## Usage

```bash
# Dump MXF header metadata as RegXML
npx regxmllib-rs dump -d Elements.xml Groups.xml Types.xml -i video.mxf

# Convert SMPTE register XMLs to metadictionary
npx regxmllib-rs xml-registers-to-dict Elements.xml Groups.xml Types.xml -o metadict.xml

# Generate XSD from metadictionary
npx regxmllib-rs gen-dict-xsd -d metadict.xml -o schema.xsd

# Help
npx regxmllib-rs --help
npx regxmllib-rs dump --help
```

## Supported platforms

| Platform | Package |
|---|---|
| macOS x64 | `regxmllib-rs-darwin-x64` |
| macOS arm64 | `regxmllib-rs-darwin-arm64` |
| Linux x64 | `regxmllib-rs-linux-x64` |
| Linux arm64 | `regxmllib-rs-linux-arm64` |
| Windows x64 | `regxmllib-rs-win32-x64` |
| Windows arm64 | `regxmllib-rs-win32-arm64` |

Platform packages are installed automatically as optional dependencies.

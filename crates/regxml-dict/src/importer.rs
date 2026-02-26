//! SMPTE register XML importer.
//!
//! Parses Types, Elements, Groups, and Labels register XMLs (produced by the
//! SMPTE Metadata Registry) into a [`MetaDictionary`] using `roxmltree`.
//!
//! Register namespaces:
//! - TypesRegister:    `http://www.smpte-ra.org/schemas/2003/2012`
//! - ElementsRegister: `http://www.smpte-ra.org/schemas/335/2012`
//! - GroupsRegister:   `http://www.smpte-ra.org/ns/395/2016`
//! - LabelsRegister:   `http://www.smpte-ra.org/schemas/400/2012`

use std::collections::HashMap;

use smpte_types::Auid;

use crate::{
    definition::{
        CharacterTypeDef, ClassDefinition, Definition, EnumerationElement, EnumerationTypeDef,
        ExtEnumTypeDef, FixedArrayTypeDef, FloatTypeDef, IndirectTypeDef, IntegerTypeDef,
        LensSerialFloatTypeDef, OpaqueTypeDef, PropertyDefinition, RecordMember, RecordTypeDef,
        RenameTypeDef, SetTypeDef, StreamTypeDef, StringTypeDef, StrongReferenceTypeDef,
        TypeDefinition, VariableArrayTypeDef, WeakReferenceTypeDef,
    },
    DictError, MetaDictionary,
};

// ─── Public API ──────────────────────────────────────────────────────────────

/// Parse one or more SMPTE register XML byte slices into a [`MetaDictionary`].
///
/// Accepts any combination of Types, Elements, Groups, and Labels registers.
/// The Elements register must be provided together with the Groups register so
/// that property symbols and types can be resolved from group contents.
pub fn import_registers(xml_sources: &[&[u8]]) -> Result<MetaDictionary, DictError> {
    // Decode all sources to UTF-8 first so we can parse multiple times.
    let texts: Vec<String> = xml_sources
        .iter()
        .map(|b| {
            std::str::from_utf8(b)
                .map(str::to_owned)
                .map_err(|e| DictError::Xml(e.to_string()))
        })
        .collect::<Result<_, _>>()?;

    // ── Pass 1: build element index from ElementsRegister ────────────────────
    // Elements entries give us symbol, namespace, and property type UL for each
    // property UL. This index is used when processing Group Contents.Record lists.
    let mut element_index: HashMap<Auid, ElementEntry> = HashMap::new();
    for text in &texts {
        let doc = parse_doc(text)?;
        if doc.root_element().tag_name().name() == "ElementsRegister" {
            collect_elements(&doc, &mut element_index)?;
        }
    }

    let mut dict = MetaDictionary::new("", "");

    // ── Pass 2: parse TypesRegister → TypeDefinitions ────────────────────────
    for text in &texts {
        let doc = parse_doc(text)?;
        if doc.root_element().tag_name().name() == "TypesRegister" {
            parse_types(&doc, &mut dict)?;
        }
    }

    // ── Pass 3: parse GroupsRegister → ClassDefinitions + PropertyDefinitions ─
    for text in &texts {
        let doc = parse_doc(text)?;
        if doc.root_element().tag_name().name() == "GroupsRegister" {
            parse_groups(&doc, &element_index, &mut dict)?;
        }
    }

    Ok(dict)
}

// ─── Internal helpers ────────────────────────────────────────────────────────

/// Minimal property info harvested from an Elements register entry.
struct ElementEntry {
    symbol: String,
    namespace: String,
    type_auid: Auid,
}

fn parse_doc(text: &str) -> Result<roxmltree::Document<'_>, DictError> {
    roxmltree::Document::parse(text).map_err(|e| DictError::Xml(e.to_string()))
}

/// Iterate over LEAF `<Entry>` children of `<Entries>`.
fn leaf_entries<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
) -> impl Iterator<Item = roxmltree::Node<'a, 'input>> {
    doc.root_element()
        .children()
        .find(|n| n.tag_name().name() == "Entries")
        .into_iter()
        .flat_map(|entries| entries.children())
        .filter(|n| {
            n.tag_name().name() == "Entry"
                && child_text(n, "Kind").as_deref() != Some("NODE")
        })
}

/// Get text of a named child element (first match).
fn child_text(node: &roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.children()
        .find(|n| n.tag_name().name() == name)
        .and_then(|n| n.text())
        .map(str::to_owned)
}

/// Parse a required child text value.
fn req(node: &roxmltree::Node<'_, '_>, field: &str, ctx: &str) -> Result<String, DictError> {
    child_text(node, field)
        .ok_or_else(|| DictError::Xml(format!("{ctx}: missing <{field}>")))
}

/// Parse an `Auid` from a required child text element containing a URN.
fn req_auid(node: &roxmltree::Node<'_, '_>, field: &str, ctx: &str) -> Result<Auid, DictError> {
    let s = req(node, field, ctx)?;
    Auid::from_urn(&s).map_err(|e| DictError::Xml(format!("{ctx}: <{field}>: {e}")))
}

/// Parse an optional `Auid` from a child text element.
fn opt_auid(
    node: &roxmltree::Node<'_, '_>,
    field: &str,
) -> Result<Option<Auid>, DictError> {
    match child_text(node, field) {
        None => Ok(None),
        Some(s) => Auid::from_urn(&s)
            .map(Some)
            .map_err(|e| DictError::Xml(e.to_string())),
    }
}

// ─── ElementsRegister ────────────────────────────────────────────────────────

fn collect_elements(
    doc: &roxmltree::Document<'_>,
    index: &mut HashMap<Auid, ElementEntry>,
) -> Result<(), DictError> {
    for entry in leaf_entries(doc) {
        let ul_str = match child_text(&entry, "UL") {
            Some(s) => s,
            None => continue,
        };
        let ident = match Auid::from_urn(&ul_str) {
            Ok(a) => a,
            Err(_) => continue, // skip malformed
        };
        let symbol = match child_text(&entry, "Symbol") {
            Some(s) => s,
            None => continue,
        };
        let namespace = child_text(&entry, "NamespaceName").unwrap_or_default();
        let type_auid = match opt_auid(&entry, "Type")? {
            Some(a) => a,
            None => continue, // Elements without Type are not property definitions
        };
        index.insert(
            ident,
            ElementEntry { symbol, namespace, type_auid },
        );
    }
    Ok(())
}

// ─── TypesRegister ───────────────────────────────────────────────────────────

fn parse_types(doc: &roxmltree::Document<'_>, dict: &mut MetaDictionary) -> Result<(), DictError> {
    for entry in leaf_entries(doc) {
        let ul_str = match child_text(&entry, "UL") {
            Some(s) => s,
            None => continue,
        };
        let ident = match Auid::from_urn(&ul_str) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let symbol = match child_text(&entry, "Symbol") {
            Some(s) => s,
            None => continue,
        };
        let namespace = child_text(&entry, "NamespaceName").unwrap_or_default();
        let type_kind = match child_text(&entry, "TypeKind") {
            Some(s) => s,
            None => continue, // entries without TypeKind are not type definitions
        };

        let ctx = format!("{type_kind}:{ul_str}");

        let td: TypeDefinition = match type_kind.as_str() {
            "Integer" => {
                let size = child_text(&entry, "TypeSize")
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(1);
                let qualifiers = child_text(&entry, "TypeQualifiers").unwrap_or_default();
                let is_signed = qualifiers.contains("isSigned");
                TypeDefinition::Integer(IntegerTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    size,
                    is_signed,
                })
            }

            "Rename" => {
                let renamed_type = req_auid(&entry, "BaseType", &ctx)?;
                TypeDefinition::Rename(RenameTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    renamed_type,
                })
            }

            "FixedArray" => {
                let element_type = req_auid(&entry, "BaseType", &ctx)?;
                let element_count = child_text(&entry, "TypeSize")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                TypeDefinition::FixedArray(FixedArrayTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    element_type,
                    element_count,
                })
            }

            "VariableArray" => {
                let element_type = req_auid(&entry, "BaseType", &ctx)?;
                TypeDefinition::VariableArray(VariableArrayTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    element_type,
                })
            }

            "Set" => {
                let element_type = req_auid(&entry, "BaseType", &ctx)?;
                TypeDefinition::Set(SetTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    element_type,
                })
            }

            "String" => {
                let element_type = req_auid(&entry, "BaseType", &ctx)?;
                TypeDefinition::String(StringTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    element_type,
                })
            }

            "Character" => {
                TypeDefinition::Character(CharacterTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                })
            }

            "Stream" => {
                TypeDefinition::Stream(StreamTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                })
            }

            "Indirect" => {
                TypeDefinition::Indirect(IndirectTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                })
            }

            "Opaque" => {
                TypeDefinition::Opaque(OpaqueTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                })
            }

            "Float" => {
                let size = child_text(&entry, "TypeSize")
                    .and_then(|s| s.parse::<u8>().ok())
                    .unwrap_or(4);
                TypeDefinition::Float(FloatTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    size,
                })
            }

            "LensSerialFloat" => {
                TypeDefinition::LensSerialFloat(LensSerialFloatTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                })
            }

            "StrongReference" => {
                let referenced_type = req_auid(&entry, "BaseType", &ctx)?;
                TypeDefinition::StrongReference(StrongReferenceTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    referenced_type,
                })
            }

            "WeakReference" => {
                let referenced_type = req_auid(&entry, "BaseType", &ctx)?;
                let target_set = parse_facet_values_as_auids(&entry)?;
                TypeDefinition::WeakReference(WeakReferenceTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    referenced_type,
                    target_set,
                })
            }

            "Record" => {
                let members = parse_record_facets(&entry, &ctx)?;
                TypeDefinition::Record(RecordTypeDef {
                    identification: ident,
                    symbol,
                    namespace,
                    members,
                })
            }

            "Enumeration" => {
                // Distinguish regular Enumeration from ExtendibleEnumeration:
                // If facet Values are UL URNs → ExtendibleEnumeration;
                // if they are integers → Enumeration.
                let facets = facets_node(&entry);
                let first_value = facets
                    .and_then(|f| {
                        f.children()
                            .find(|n| n.tag_name().name() == "Facet")
                    })
                    .and_then(|facet| child_text(&facet, "Value"));

                if first_value
                    .as_deref()
                    .map(|v| v.starts_with("urn:"))
                    .unwrap_or(false)
                {
                    // ExtendibleEnumeration — elements are external Labels; leave empty.
                    TypeDefinition::ExtendibleEnumeration(ExtEnumTypeDef {
                        identification: ident,
                        symbol,
                        namespace,
                        elements: Vec::new(),
                    })
                } else {
                    let element_type = req_auid(&entry, "BaseType", &ctx)?;
                    let elements = parse_enum_facets(&entry)?;
                    TypeDefinition::Enumeration(EnumerationTypeDef {
                        identification: ident,
                        symbol,
                        namespace,
                        element_type,
                        elements,
                    })
                }
            }

            other => {
                // Unknown TypeKind — skip silently to be forward-compatible.
                let _ = other;
                continue;
            }
        };

        dict.add(Definition::Type(td))?;
    }
    Ok(())
}

/// Return the `<Facets>` child node of `entry`, if present.
fn facets_node<'a, 'input>(
    entry: &'a roxmltree::Node<'a, 'input>,
) -> Option<roxmltree::Node<'a, 'input>> {
    entry
        .children()
        .find(|n| n.tag_name().name() == "Facets")
}

/// Collect `<Facets><Facet><Value>` as AUIDs (for WeakReference target sets).
fn parse_facet_values_as_auids(entry: &roxmltree::Node<'_, '_>) -> Result<Vec<Auid>, DictError> {
    let mut result = Vec::new();
    if let Some(facets) = facets_node(entry) {
        for facet in facets.children().filter(|n| n.tag_name().name() == "Facet") {
            if let Some(v) = child_text(&facet, "Value") {
                if let Ok(a) = Auid::from_urn(&v) {
                    result.push(a);
                }
            }
        }
    }
    Ok(result)
}

/// Collect `<Facets><Facet>` as Record members (Symbol + Type UL).
fn parse_record_facets(
    entry: &roxmltree::Node<'_, '_>,
    ctx: &str,
) -> Result<Vec<RecordMember>, DictError> {
    let mut members = Vec::new();
    if let Some(facets) = facets_node(entry) {
        for facet in facets.children().filter(|n| n.tag_name().name() == "Facet") {
            let name = match child_text(&facet, "Symbol") {
                Some(s) => s,
                None => continue,
            };
            let field_type = req_auid(&facet, "Type", ctx)?;
            members.push(RecordMember { name, field_type });
        }
    }
    Ok(members)
}

/// Collect `<Facets><Facet>` as Enumeration elements (Symbol + integer Value).
fn parse_enum_facets(entry: &roxmltree::Node<'_, '_>) -> Result<Vec<EnumerationElement>, DictError> {
    let mut elements = Vec::new();
    if let Some(facets) = facets_node(entry) {
        for facet in facets.children().filter(|n| n.tag_name().name() == "Facet") {
            let name = match child_text(&facet, "Symbol") {
                Some(s) => s,
                None => continue,
            };
            let value = child_text(&facet, "Value")
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            elements.push(EnumerationElement { name, value });
        }
    }
    Ok(elements)
}

// ─── GroupsRegister ──────────────────────────────────────────────────────────

fn parse_groups(
    doc: &roxmltree::Document<'_>,
    element_index: &HashMap<Auid, ElementEntry>,
    dict: &mut MetaDictionary,
) -> Result<(), DictError> {
    for entry in leaf_entries(doc) {
        let ul_str = match child_text(&entry, "UL") {
            Some(s) => s,
            None => continue,
        };
        let ident = match Auid::from_urn(&ul_str) {
            Ok(a) => a,
            Err(_) => continue,
        };
        let symbol = match child_text(&entry, "Symbol") {
            Some(s) => s,
            None => continue,
        };
        let namespace = child_text(&entry, "NamespaceName").unwrap_or_default();
        let parent_class = opt_auid(&entry, "Parent")?;
        let is_concrete = child_text(&entry, "IsConcrete")
            .as_deref()
            .map(parse_bool)
            .unwrap_or(true);

        dict.add(Definition::Class(ClassDefinition {
            identification: ident,
            symbol,
            namespace,
            parent_class,
            is_concrete,
        }))?;

        // Parse Contents.Record → PropertyDefinitions
        if let Some(contents) = entry.children().find(|n| n.tag_name().name() == "Contents") {
            for record in contents.children().filter(|n| n.tag_name().name() == "Record") {
                if let Some(prop_def) =
                    parse_property_from_record(&record, &ident, element_index)?
                {
                    dict.add(Definition::Property(prop_def))?;
                }
            }
        }
    }
    Ok(())
}

/// Build a `PropertyDefinition` from a Groups `<Contents><Record>` entry.
///
/// Returns `None` if the property element UL is not in the element index
/// (e.g. a proprietary / dark property).
fn parse_property_from_record(
    record: &roxmltree::Node<'_, '_>,
    member_of: &Auid,
    element_index: &HashMap<Auid, ElementEntry>,
) -> Result<Option<PropertyDefinition>, DictError> {
    let prop_ul_str = match child_text(record, "UL") {
        Some(s) => s,
        None => return Ok(None),
    };
    let prop_ident = match Auid::from_urn(&prop_ul_str) {
        Ok(a) => a,
        Err(_) => return Ok(None),
    };

    // Look up element info
    let elem = match element_index.get(&prop_ident) {
        Some(e) => e,
        None => return Ok(None), // unknown / proprietary element
    };

    let is_optional = child_text(record, "IsOptional")
        .as_deref()
        .map(parse_bool)
        .unwrap_or(true);
    let is_unique = child_text(record, "IsUniqueID")
        .as_deref()
        .map(parse_bool)
        .unwrap_or(false);
    let local_id: u16 = child_text(record, "LocalTag")
        .as_deref()
        .and_then(|s| u16::from_str_radix(s.trim(), 16).ok())
        .unwrap_or(0);

    Ok(Some(PropertyDefinition {
        identification: prop_ident,
        symbol: elem.symbol.clone(),
        namespace: elem.namespace.clone(),
        member_of: *member_of,
        property_type: elem.type_auid,
        is_optional,
        is_unique_identifier: is_unique,
        local_identification: local_id,
    }))
}

// ─── Utility ─────────────────────────────────────────────────────────────────

fn parse_bool(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")
}

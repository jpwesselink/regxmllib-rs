use std::collections::HashMap;

use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use smpte_types::Auid;

use crate::{
    definition::{
        CharacterTypeDef, ClassDefinition, Definition, EnumerationElement, EnumerationTypeDef,
        ExtEnumTypeDef, FixedArrayTypeDef, FloatTypeDef, IndirectTypeDef, IntegerTypeDef,
        LensSerialFloatTypeDef, OpaqueTypeDef, PropertyDefinition, RecordMember, RecordTypeDef,
        RenameTypeDef, SetTypeDef, StreamTypeDef, StringTypeDef, StrongReferenceTypeDef,
        TypeDefinition, VariableArrayTypeDef, WeakReferenceTypeDef,
    },
    resolver::DefinitionResolver,
    DictError,
};

const NS: &str = "http://www.smpte-ra.org/schemas/2001-1b/2013/metadict";

// ── MetaDictionary ──────────────────────────────────────────────────────────

/// A single RegXML metadictionary (SMPTE ST 2001-1 Annex A).
#[derive(Debug, Default)]
pub struct MetaDictionary {
    pub scheme_id: String,
    pub scheme_uri: String,
    definitions_by_auid: HashMap<Auid, Definition>,
    definitions_by_symbol: HashMap<String, Auid>,
    /// class AUID → property AUIDs
    members_of: HashMap<Auid, Vec<Auid>>,
    /// parent class AUID → child class AUIDs
    subclasses_of: HashMap<Auid, Vec<Auid>>,
}

impl MetaDictionary {
    pub fn new(scheme_uri: impl Into<String>, scheme_id: impl Into<String>) -> Self {
        MetaDictionary {
            scheme_uri: scheme_uri.into(),
            scheme_id: scheme_id.into(),
            ..Default::default()
        }
    }

    // ── fromXML ─────────────────────────────────────────────────────────────

    pub fn from_xml(xml: &[u8]) -> Result<Self, DictError> {
        let text = std::str::from_utf8(xml).map_err(|e| DictError::Xml(e.to_string()))?;
        let doc = roxmltree::Document::parse(text).map_err(|e| DictError::Xml(e.to_string()))?;
        let root = doc.root_element(); // <Extension>

        let scheme_id = child_text(&root, "SchemeID").unwrap_or_default();
        let scheme_uri = child_text(&root, "SchemeURI").unwrap_or_default();
        let mut dict = MetaDictionary::new(scheme_uri, scheme_id);

        if let Some(meta_defs) = root
            .children()
            .find(|n| n.tag_name().name() == "MetaDefinitions")
        {
            for node in meta_defs.children().filter(|n| n.is_element()) {
                if let Some(def) = parse_meta_definition(&node)
                    .map_err(|e| DictError::Xml(format!("in <{}>: {e}", node.tag_name().name())))?
                {
                    dict.add(def)?;
                }
            }
        }
        Ok(dict)
    }

    // ── add ──────────────────────────────────────────────────────────────────

    pub fn add(&mut self, def: Definition) -> Result<(), DictError> {
        let id = *def.identification();
        let sym = def.symbol().to_owned();

        // Build reverse-lookup indexes before inserting
        match &def {
            Definition::Property(p) => {
                self.members_of.entry(p.member_of).or_default().push(id);
            }
            Definition::Class(c) => {
                if let Some(parent) = c.parent_class {
                    self.subclasses_of.entry(parent).or_default().push(id);
                }
            }
            Definition::Type(_) => {}
        }

        self.definitions_by_symbol.insert(sym, id);
        self.definitions_by_auid.insert(id, def);
        Ok(())
    }

    // ── accessors ─────────────────────────────────────────────────────────────

    pub fn definition_count(&self) -> usize {
        self.definitions_by_auid.len()
    }

    pub fn all_definitions(&self) -> impl Iterator<Item = &Definition> {
        self.definitions_by_auid.values()
    }

    pub fn get_definition_by_symbol(&self, symbol: &str) -> Option<&Definition> {
        self.definitions_by_symbol
            .get(symbol)
            .and_then(|id| self.definitions_by_auid.get(id))
    }

    // ── to_xml ────────────────────────────────────────────────────────────────

    pub fn to_xml(&self) -> Result<Vec<u8>, DictError> {
        let mut buf = Vec::new();
        let mut w = Writer::new_with_indent(&mut buf, b' ', 2);

        w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| DictError::Xml(e.to_string()))?;

        let mut root = BytesStart::new("Extension");
        root.push_attribute(("xmlns", NS));
        w.write_event(Event::Start(root))
            .map_err(|e| DictError::Xml(e.to_string()))?;

        write_simple(&mut w, "SchemeID", &self.scheme_id)?;
        write_simple(&mut w, "SchemeURI", &self.scheme_uri)?;

        w.write_event(Event::Start(BytesStart::new("MetaDefinitions")))
            .map_err(|e| DictError::Xml(e.to_string()))?;

        for def in self.definitions_by_auid.values() {
            write_definition(&mut w, def)?;
        }

        w.write_event(Event::End(BytesEnd::new("MetaDefinitions")))
            .map_err(|e| DictError::Xml(e.to_string()))?;
        w.write_event(Event::End(BytesEnd::new("Extension")))
            .map_err(|e| DictError::Xml(e.to_string()))?;

        Ok(buf)
    }
}

impl DefinitionResolver for MetaDictionary {
    fn get_definition(&self, id: &Auid) -> Option<&Definition> {
        self.definitions_by_auid.get(id)
    }
    fn get_subclasses_of(&self, class: &ClassDefinition) -> Vec<Auid> {
        self.subclasses_of
            .get(&class.identification)
            .cloned()
            .unwrap_or_default()
    }
    fn get_members_of(&self, class: &ClassDefinition) -> Vec<Auid> {
        self.members_of
            .get(&class.identification)
            .cloned()
            .unwrap_or_default()
    }
}

// ── MetaDictionaryCollection ─────────────────────────────────────────────────

/// Aggregates multiple [`MetaDictionary`] instances with a merged lookup index.
#[derive(Debug, Default)]
pub struct MetaDictionaryCollection {
    dictionaries: Vec<MetaDictionary>,
    index: HashMap<Auid, usize>,
}

impl MetaDictionaryCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, dict: MetaDictionary) -> Result<(), DictError> {
        let idx = self.dictionaries.len();
        for id in dict.definitions_by_auid.keys() {
            self.index.entry(*id).or_insert(idx);
        }
        self.dictionaries.push(dict);
        Ok(())
    }

    pub fn from_xml_slices(slices: &[&[u8]]) -> Result<Self, DictError> {
        let mut coll = Self::new();
        for xml in slices {
            coll.add(MetaDictionary::from_xml(xml)?)?;
        }
        Ok(coll)
    }

    pub fn get_definition_by_symbol(&self, symbol: &str) -> Option<&Definition> {
        self.dictionaries
            .iter()
            .find_map(|d| d.get_definition_by_symbol(symbol))
    }
}

impl DefinitionResolver for MetaDictionaryCollection {
    fn get_definition(&self, id: &Auid) -> Option<&Definition> {
        self.index
            .get(id)
            .and_then(|&i| self.dictionaries.get(i))
            .and_then(|d| d.definitions_by_auid.get(id))
    }
    fn get_subclasses_of(&self, class: &ClassDefinition) -> Vec<Auid> {
        self.dictionaries
            .iter()
            .flat_map(|d| d.get_subclasses_of(class))
            .collect()
    }
    fn get_members_of(&self, class: &ClassDefinition) -> Vec<Auid> {
        self.dictionaries
            .iter()
            .flat_map(|d| d.get_members_of(class))
            .collect()
    }
}

// ── Parsing helpers ───────────────────────────────────────────────────────────

fn child_text(node: &roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    node.children()
        .find(|n| n.tag_name().name() == name)
        .and_then(|n| n.text())
        .map(str::to_owned)
}

fn required_auid(
    node: &roxmltree::Node<'_, '_>,
    field: &'static str,
    ctx: &str,
) -> Result<Auid, DictError> {
    let s =
        child_text(node, field).ok_or_else(|| DictError::MissingField(field, ctx.to_owned()))?;
    Auid::from_urn(&s).map_err(|e| DictError::Xml(format!("bad AUID in <{field}> of {ctx}: {e}")))
}

fn optional_auid(node: &roxmltree::Node<'_, '_>, field: &str) -> Result<Option<Auid>, DictError> {
    if let Some(s) = child_text(node, field) {
        Ok(Some(Auid::from_urn(&s).map_err(|e| {
            DictError::Xml(format!("bad AUID in <{field}>: {e}"))
        })?))
    } else {
        Ok(None)
    }
}

fn required_str(
    node: &roxmltree::Node<'_, '_>,
    field: &'static str,
    ctx: &str,
) -> Result<String, DictError> {
    child_text(node, field).ok_or_else(|| DictError::MissingField(field, ctx.to_owned()))
}

fn parse_bool(s: &str) -> bool {
    s.eq_ignore_ascii_case("true")
}

// ── Per-element-name parsers ─────────────────────────────────────────────────

fn parse_meta_definition(node: &roxmltree::Node<'_, '_>) -> Result<Option<Definition>, DictError> {
    let tag = node.tag_name().name();
    let id_str = match child_text(node, "Identification") {
        Some(s) => s,
        None => return Ok(None), // ignore entries without Identification
    };
    let ctx = format!("{tag}:{id_str}");

    let ident = Auid::from_urn(&id_str).map_err(|e| DictError::Xml(format!("{ctx}: {e}")))?;
    let symbol = required_str(node, "Symbol", &ctx)?;
    let _name = child_text(node, "Name").unwrap_or_default();

    match tag {
        "ClassDefinition" => {
            let parent_class = optional_auid(node, "ParentClass")?;
            let is_concrete = child_text(node, "IsConcrete")
                .as_deref()
                .map(parse_bool)
                .unwrap_or(true);
            Ok(Some(Definition::Class(ClassDefinition {
                identification: ident,
                symbol,
                namespace: String::new(),
                parent_class,
                is_concrete,
            })))
        }

        "PropertyDefinition" => {
            let prop_type = required_auid(node, "Type", &ctx)?;
            let member_of = required_auid(node, "MemberOf", &ctx)?;
            let is_optional = child_text(node, "IsOptional")
                .as_deref()
                .map(parse_bool)
                .unwrap_or(false);
            let is_unique = child_text(node, "IsUniqueIdentifier")
                .as_deref()
                .map(parse_bool)
                .unwrap_or(false);
            let local_id: u16 = child_text(node, "LocalIdentification")
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            Ok(Some(Definition::Property(PropertyDefinition {
                identification: ident,
                symbol,
                namespace: String::new(),
                member_of,
                property_type: prop_type,
                is_optional,
                is_unique_identifier: is_unique,
                local_identification: local_id,
            })))
        }

        "TypeDefinitionInteger" => {
            let size = child_text(node, "Size")
                .and_then(|s| s.parse::<u8>().ok())
                .unwrap_or(1);
            let is_signed = child_text(node, "IsSigned")
                .as_deref()
                .map(parse_bool)
                .unwrap_or(false);
            Ok(Some(Definition::Type(TypeDefinition::Integer(
                IntegerTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    size,
                    is_signed,
                },
            ))))
        }

        "TypeDefinitionRename" => {
            let renamed = required_auid(node, "RenamedType", &ctx)?;
            Ok(Some(Definition::Type(TypeDefinition::Rename(
                RenameTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    renamed_type: renamed,
                },
            ))))
        }

        "TypeDefinitionRecord" => {
            let members = parse_record_members(node)?;
            Ok(Some(Definition::Type(TypeDefinition::Record(
                RecordTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    members,
                },
            ))))
        }

        "TypeDefinitionEnumeration" => {
            let element_type = required_auid(node, "ElementType", &ctx)?;
            let elements = parse_enum_elements(node)?;
            Ok(Some(Definition::Type(TypeDefinition::Enumeration(
                EnumerationTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    element_type,
                    elements,
                },
            ))))
        }

        "TypeDefinitionExtendibleEnumeration" => {
            let elements = parse_enum_elements(node)?;
            Ok(Some(Definition::Type(
                TypeDefinition::ExtendibleEnumeration(ExtEnumTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    elements,
                }),
            )))
        }

        "TypeDefinitionFixedArray" => {
            let element_type = required_auid(node, "ElementType", &ctx)?;
            let element_count = child_text(node, "ElementCount")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            Ok(Some(Definition::Type(TypeDefinition::FixedArray(
                FixedArrayTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    element_type,
                    element_count,
                },
            ))))
        }

        "TypeDefinitionVariableArray" => {
            let element_type = required_auid(node, "ElementType", &ctx)?;
            Ok(Some(Definition::Type(TypeDefinition::VariableArray(
                VariableArrayTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    element_type,
                },
            ))))
        }

        "TypeDefinitionSet" => {
            let element_type = required_auid(node, "ElementType", &ctx)?;
            Ok(Some(Definition::Type(TypeDefinition::Set(SetTypeDef {
                identification: ident,
                symbol,
                namespace: String::new(),
                element_type,
            }))))
        }

        "TypeDefinitionString" => {
            let element_type = required_auid(node, "ElementType", &ctx)?;
            Ok(Some(Definition::Type(TypeDefinition::String(
                StringTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    element_type,
                },
            ))))
        }

        "TypeDefinitionCharacter" => Ok(Some(Definition::Type(TypeDefinition::Character(
            CharacterTypeDef {
                identification: ident,
                symbol,
                namespace: String::new(),
            },
        )))),

        "TypeDefinitionStream" => Ok(Some(Definition::Type(TypeDefinition::Stream(
            StreamTypeDef {
                identification: ident,
                symbol,
                namespace: String::new(),
            },
        )))),

        "TypeDefinitionIndirect" => Ok(Some(Definition::Type(TypeDefinition::Indirect(
            IndirectTypeDef {
                identification: ident,
                symbol,
                namespace: String::new(),
            },
        )))),

        "TypeDefinitionOpaque" => Ok(Some(Definition::Type(TypeDefinition::Opaque(
            OpaqueTypeDef {
                identification: ident,
                symbol,
                namespace: String::new(),
            },
        )))),

        "TypeDefinitionStrongObjectReference" => {
            let referenced = required_auid(node, "ReferencedType", &ctx)?;
            Ok(Some(Definition::Type(TypeDefinition::StrongReference(
                StrongReferenceTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    referenced_type: referenced,
                },
            ))))
        }

        "TypeDefinitionWeakObjectReference" => {
            let referenced = required_auid(node, "ReferencedType", &ctx)?;
            let target_set = parse_target_set(node)?;
            Ok(Some(Definition::Type(TypeDefinition::WeakReference(
                WeakReferenceTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    referenced_type: referenced,
                    target_set,
                },
            ))))
        }

        // Float is not in the standard dict files but may appear in extensions
        "TypeDefinitionFloat" => {
            let size = child_text(node, "Size")
                .and_then(|s| s.parse::<u8>().ok())
                .unwrap_or(4);
            Ok(Some(Definition::Type(TypeDefinition::Float(
                FloatTypeDef {
                    identification: ident,
                    symbol,
                    namespace: String::new(),
                    size,
                },
            ))))
        }

        "TypeDefinitionLensSerialFloat" => Ok(Some(Definition::Type(
            TypeDefinition::LensSerialFloat(LensSerialFloatTypeDef {
                identification: ident,
                symbol,
                namespace: String::new(),
            }),
        ))),

        _ => Ok(None), // unknown — skip gracefully
    }
}

/// Parse `<Members>` as alternating `<Name>` / `<Type>` children.
fn parse_record_members(node: &roxmltree::Node<'_, '_>) -> Result<Vec<RecordMember>, DictError> {
    let members_node = match node.children().find(|n| n.tag_name().name() == "Members") {
        Some(n) => n,
        None => return Ok(vec![]),
    };

    let mut names: Vec<String> = Vec::new();
    let mut types: Vec<String> = Vec::new();
    for child in members_node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "Name" => names.push(child.text().unwrap_or_default().to_owned()),
            "Type" => types.push(child.text().unwrap_or_default().to_owned()),
            _ => {}
        }
    }
    names
        .into_iter()
        .zip(types)
        .map(|(n, t)| {
            let field_type = Auid::from_urn(&t)
                .map_err(|e| DictError::Xml(format!("bad member type {t:?}: {e}")))?;
            Ok(RecordMember {
                name: n,
                field_type,
            })
        })
        .collect()
}

/// Parse `<Elements>` as alternating `<Name>` / `<Value>` children.
fn parse_enum_elements(
    node: &roxmltree::Node<'_, '_>,
) -> Result<Vec<EnumerationElement>, DictError> {
    let elems_node = match node.children().find(|n| n.tag_name().name() == "Elements") {
        Some(n) => n,
        None => return Ok(vec![]),
    };

    let mut names: Vec<String> = Vec::new();
    let mut values: Vec<i64> = Vec::new();
    for child in elems_node.children().filter(|n| n.is_element()) {
        match child.tag_name().name() {
            "Name" => names.push(child.text().unwrap_or_default().to_owned()),
            "Value" => {
                let v = child
                    .text()
                    .unwrap_or("0")
                    .trim()
                    .parse::<i64>()
                    .unwrap_or(0);
                values.push(v);
            }
            _ => {}
        }
    }
    Ok(names
        .into_iter()
        .zip(values)
        .map(|(name, value)| EnumerationElement { name, value })
        .collect())
}

/// Parse `<TargetSet><MetaDefRef>…</MetaDefRef>…</TargetSet>`.
fn parse_target_set(node: &roxmltree::Node<'_, '_>) -> Result<Vec<Auid>, DictError> {
    let ts = match node.children().find(|n| n.tag_name().name() == "TargetSet") {
        Some(n) => n,
        None => return Ok(vec![]),
    };
    ts.children()
        .filter(|n| n.tag_name().name() == "MetaDefRef")
        .filter_map(|n| n.text())
        .map(|s| Auid::from_urn(s).map_err(|e| DictError::Xml(e.to_string())))
        .collect()
}

// ── XML serialisation helpers ─────────────────────────────────────────────────

fn write_simple<W: std::io::Write>(
    w: &mut Writer<W>,
    tag: &str,
    text: &str,
) -> Result<(), DictError> {
    w.write_event(Event::Start(BytesStart::new(tag)))
        .and_then(|_| w.write_event(Event::Text(BytesText::new(text))))
        .and_then(|_| w.write_event(Event::End(BytesEnd::new(tag))))
        .map_err(|e| DictError::Xml(e.to_string()))
}

fn write_definition<W: std::io::Write>(
    w: &mut Writer<W>,
    def: &Definition,
) -> Result<(), DictError> {
    match def {
        Definition::Class(c) => write_class(w, c),
        Definition::Property(p) => write_property(w, p),
        Definition::Type(t) => write_type(w, t),
    }
}

fn write_class<W: std::io::Write>(w: &mut Writer<W>, c: &ClassDefinition) -> Result<(), DictError> {
    w.write_event(Event::Start(BytesStart::new("ClassDefinition")))
        .map_err(|e| DictError::Xml(e.to_string()))?;
    write_simple(w, "Identification", &c.identification.to_string())?;
    write_simple(w, "Symbol", &c.symbol)?;
    write_simple(w, "Name", "")?;
    if let Some(p) = c.parent_class {
        write_simple(w, "ParentClass", &p.to_string())?;
    }
    write_simple(
        w,
        "IsConcrete",
        if c.is_concrete { "true" } else { "false" },
    )?;
    w.write_event(Event::End(BytesEnd::new("ClassDefinition")))
        .map_err(|e| DictError::Xml(e.to_string()))
}

fn write_property<W: std::io::Write>(
    w: &mut Writer<W>,
    p: &PropertyDefinition,
) -> Result<(), DictError> {
    w.write_event(Event::Start(BytesStart::new("PropertyDefinition")))
        .map_err(|e| DictError::Xml(e.to_string()))?;
    write_simple(w, "Identification", &p.identification.to_string())?;
    write_simple(w, "Symbol", &p.symbol)?;
    write_simple(w, "Name", "")?;
    write_simple(w, "Type", &p.property_type.to_string())?;
    write_simple(
        w,
        "IsOptional",
        if p.is_optional { "true" } else { "false" },
    )?;
    write_simple(
        w,
        "IsUniqueIdentifier",
        if p.is_unique_identifier {
            "true"
        } else {
            "false"
        },
    )?;
    write_simple(
        w,
        "LocalIdentification",
        &p.local_identification.to_string(),
    )?;
    write_simple(w, "MemberOf", &p.member_of.to_string())?;
    w.write_event(Event::End(BytesEnd::new("PropertyDefinition")))
        .map_err(|e| DictError::Xml(e.to_string()))
}

fn write_type<W: std::io::Write>(w: &mut Writer<W>, t: &TypeDefinition) -> Result<(), DictError> {
    let (tag, id, sym): (&str, &Auid, &str) = match t {
        TypeDefinition::Integer(d) => ("TypeDefinitionInteger", &d.identification, &d.symbol),
        TypeDefinition::Rename(d) => ("TypeDefinitionRename", &d.identification, &d.symbol),
        TypeDefinition::Record(d) => ("TypeDefinitionRecord", &d.identification, &d.symbol),
        TypeDefinition::Enumeration(d) => {
            ("TypeDefinitionEnumeration", &d.identification, &d.symbol)
        }
        TypeDefinition::ExtendibleEnumeration(d) => (
            "TypeDefinitionExtendibleEnumeration",
            &d.identification,
            &d.symbol,
        ),
        TypeDefinition::FixedArray(d) => ("TypeDefinitionFixedArray", &d.identification, &d.symbol),
        TypeDefinition::VariableArray(d) => {
            ("TypeDefinitionVariableArray", &d.identification, &d.symbol)
        }
        TypeDefinition::Set(d) => ("TypeDefinitionSet", &d.identification, &d.symbol),
        TypeDefinition::String(d) => ("TypeDefinitionString", &d.identification, &d.symbol),
        TypeDefinition::Character(d) => ("TypeDefinitionCharacter", &d.identification, &d.symbol),
        TypeDefinition::Stream(d) => ("TypeDefinitionStream", &d.identification, &d.symbol),
        TypeDefinition::Indirect(d) => ("TypeDefinitionIndirect", &d.identification, &d.symbol),
        TypeDefinition::Opaque(d) => ("TypeDefinitionOpaque", &d.identification, &d.symbol),
        TypeDefinition::StrongReference(d) => (
            "TypeDefinitionStrongObjectReference",
            &d.identification,
            &d.symbol,
        ),
        TypeDefinition::WeakReference(d) => (
            "TypeDefinitionWeakObjectReference",
            &d.identification,
            &d.symbol,
        ),
        TypeDefinition::Float(d) => ("TypeDefinitionFloat", &d.identification, &d.symbol),
        TypeDefinition::LensSerialFloat(d) => (
            "TypeDefinitionLensSerialFloat",
            &d.identification,
            &d.symbol,
        ),
    };

    w.write_event(Event::Start(BytesStart::new(tag)))
        .map_err(|e| DictError::Xml(e.to_string()))?;
    write_simple(w, "Identification", &id.to_string())?;
    write_simple(w, "Symbol", sym)?;

    // type-specific extra fields
    match t {
        TypeDefinition::Integer(d) => {
            write_simple(w, "Size", &d.size.to_string())?;
            write_simple(w, "IsSigned", if d.is_signed { "true" } else { "false" })?;
        }
        TypeDefinition::Rename(d) => {
            write_simple(w, "RenamedType", &d.renamed_type.to_string())?;
        }
        TypeDefinition::Record(d) => {
            w.write_event(Event::Start(BytesStart::new("Members")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
            for m in &d.members {
                write_simple(w, "Name", &m.name)?;
                write_simple(w, "Type", &m.field_type.to_string())?;
            }
            w.write_event(Event::End(BytesEnd::new("Members")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
        }
        TypeDefinition::Enumeration(d) => {
            write_simple(w, "ElementType", &d.element_type.to_string())?;
            w.write_event(Event::Start(BytesStart::new("Elements")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
            for e in &d.elements {
                write_simple(w, "Name", &e.name)?;
                write_simple(w, "Value", &e.value.to_string())?;
            }
            w.write_event(Event::End(BytesEnd::new("Elements")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
        }
        TypeDefinition::ExtendibleEnumeration(d) => {
            w.write_event(Event::Start(BytesStart::new("Elements")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
            for e in &d.elements {
                write_simple(w, "Name", &e.name)?;
                write_simple(w, "Value", &e.value.to_string())?;
            }
            w.write_event(Event::End(BytesEnd::new("Elements")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
        }
        TypeDefinition::FixedArray(d) => {
            write_simple(w, "ElementCount", &d.element_count.to_string())?;
            write_simple(w, "ElementType", &d.element_type.to_string())?;
        }
        TypeDefinition::VariableArray(d) => {
            write_simple(w, "ElementType", &d.element_type.to_string())?;
        }
        TypeDefinition::Set(d) => {
            write_simple(w, "ElementType", &d.element_type.to_string())?;
        }
        TypeDefinition::String(d) => {
            write_simple(w, "ElementType", &d.element_type.to_string())?;
        }
        TypeDefinition::StrongReference(d) => {
            write_simple(w, "ReferencedType", &d.referenced_type.to_string())?;
        }
        TypeDefinition::WeakReference(d) => {
            write_simple(w, "ReferencedType", &d.referenced_type.to_string())?;
            w.write_event(Event::Start(BytesStart::new("TargetSet")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
            for ts in &d.target_set {
                write_simple(w, "MetaDefRef", &ts.to_string())?;
            }
            w.write_event(Event::End(BytesEnd::new("TargetSet")))
                .map_err(|e| DictError::Xml(e.to_string()))?;
        }
        TypeDefinition::Float(d) => {
            write_simple(w, "Size", &d.size.to_string())?;
        }
        _ => {} // no extra fields for Character, Stream, Indirect, Opaque, LensSerialFloat
    }

    w.write_event(Event::End(BytesEnd::new(tag)))
        .map_err(|e| DictError::Xml(e.to_string()))
}

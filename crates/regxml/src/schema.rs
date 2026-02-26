//! XSD schema generation from a RegXML metadictionary (SMPTE ST 2001-1).
//!
//! Generates a single XSD file for the `reg:` baseline namespace covering:
//! - `reg:Fragment` root element
//! - `reg:Item` element (for set/array items)
//! - `reg:uid` attribute (instance UID)
//! - `<xs:import>` stubs for each class namespace found in the dictionary
//! - Named complexType + element declarations for every concrete class

use std::collections::BTreeSet;
use std::io::Write;

use quick_xml::{
    events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event},
    Writer,
};

use regxml_dict::{
    definition::{ClassDefinition, Definition, PropertyDefinition},
    DefinitionResolver, MetaDictionary,
};

use crate::{event::EventHandler, fragment::REGXML_NS, FragmentError};

const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema";

/// Generates an XSD from a [`MetaDictionary`], conforming to ST 2001-1 Rules 4–6.
pub struct XmlSchemaBuilder<'dict, R: DefinitionResolver> {
    #[allow(dead_code)]
    resolver: &'dict R,
    #[allow(dead_code)]
    handler: Option<Box<dyn EventHandler>>,
}

impl<'dict, R: DefinitionResolver> XmlSchemaBuilder<'dict, R> {
    pub fn new(resolver: &'dict R) -> Self {
        XmlSchemaBuilder {
            resolver,
            handler: None,
        }
    }

    pub fn with_event_handler(mut self, h: Box<dyn EventHandler>) -> Self {
        self.handler = Some(h);
        self
    }

    /// Write an XSD for `dict` to `writer`.
    ///
    /// Produces a single XSD targeting the RegXML baseline namespace with:
    /// - `reg:uid` attribute declaration
    /// - `reg:Fragment` element declaration
    /// - `reg:Item` element declaration
    /// - `<xs:import>` for each distinct class namespace in the dictionary
    /// - Named `complexType` + `element` declarations for every class
    pub fn from_dictionary(
        &self,
        dict: &MetaDictionary,
        writer: impl Write,
    ) -> Result<(), FragmentError> {
        let mut w = Writer::new_with_indent(writer, b' ', 2);

        // ── Collect distinct namespaces from class definitions ─────────────────
        let mut other_ns: BTreeSet<String> = BTreeSet::new();
        for def in dict.all_definitions() {
            if let Definition::Class(c) = def {
                if !c.namespace.is_empty() && c.namespace != REGXML_NS {
                    other_ns.insert(c.namespace.clone());
                }
            }
        }

        // ── xs:schema ─────────────────────────────────────────────────────────
        w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(xe)?;

        let mut schema = BytesStart::new("xs:schema");
        schema.push_attribute(("xmlns:xs", XSD_NS));
        schema.push_attribute(("xmlns:reg", REGXML_NS));
        schema.push_attribute(("targetNamespace", REGXML_NS));
        schema.push_attribute(("elementFormDefault", "qualified"));
        schema.push_attribute(("attributeFormDefault", "unqualified"));
        w.write_event(Event::Start(schema)).map_err(xe)?;

        // ── xs:import for each other namespace ─────────────────────────────────
        for ns in &other_ns {
            let mut imp = BytesStart::new("xs:import");
            imp.push_attribute(("namespace", ns.as_str()));
            w.write_event(Event::Empty(imp)).map_err(xe)?;
        }

        // ── reg:uid attribute ──────────────────────────────────────────────────
        let mut uid_attr = BytesStart::new("xs:attribute");
        uid_attr.push_attribute(("name", "uid"));
        uid_attr.push_attribute(("type", "xs:anyURI"));
        w.write_event(Event::Empty(uid_attr)).map_err(xe)?;

        // ── reg:Fragment ───────────────────────────────────────────────────────
        // <xs:element name="Fragment">
        //   <xs:complexType>
        //     <xs:sequence><xs:any namespace="##other" .../></xs:sequence>
        //   </xs:complexType>
        // </xs:element>
        {
            let mut fe = BytesStart::new("xs:element");
            fe.push_attribute(("name", "Fragment"));
            w.write_event(Event::Start(fe)).map_err(xe)?;

            w.write_event(Event::Start(BytesStart::new("xs:complexType")))
                .map_err(xe)?;
            w.write_event(Event::Start(BytesStart::new("xs:sequence")))
                .map_err(xe)?;
            let mut any = BytesStart::new("xs:any");
            any.push_attribute(("namespace", "##other"));
            any.push_attribute(("minOccurs", "0"));
            any.push_attribute(("maxOccurs", "unbounded"));
            any.push_attribute(("processContents", "lax"));
            w.write_event(Event::Empty(any)).map_err(xe)?;
            w.write_event(Event::End(BytesEnd::new("xs:sequence"))).map_err(xe)?;
            w.write_event(Event::End(BytesEnd::new("xs:complexType"))).map_err(xe)?;

            w.write_event(Event::End(BytesEnd::new("xs:element"))).map_err(xe)?;
        }

        // ── reg:Item ───────────────────────────────────────────────────────────
        // <xs:element name="Item">
        //   <xs:complexType mixed="true">
        //     <xs:sequence><xs:any .../></xs:sequence>
        //     <xs:anyAttribute .../>
        //   </xs:complexType>
        // </xs:element>
        {
            let mut ie = BytesStart::new("xs:element");
            ie.push_attribute(("name", "Item"));
            w.write_event(Event::Start(ie)).map_err(xe)?;

            let mut ct = BytesStart::new("xs:complexType");
            ct.push_attribute(("mixed", "true"));
            w.write_event(Event::Start(ct)).map_err(xe)?;

            w.write_event(Event::Start(BytesStart::new("xs:sequence")))
                .map_err(xe)?;
            let mut any = BytesStart::new("xs:any");
            any.push_attribute(("namespace", "##any"));
            any.push_attribute(("minOccurs", "0"));
            any.push_attribute(("maxOccurs", "unbounded"));
            any.push_attribute(("processContents", "lax"));
            w.write_event(Event::Empty(any)).map_err(xe)?;
            w.write_event(Event::End(BytesEnd::new("xs:sequence"))).map_err(xe)?;

            let mut aa = BytesStart::new("xs:anyAttribute");
            aa.push_attribute(("namespace", "##any"));
            aa.push_attribute(("processContents", "lax"));
            w.write_event(Event::Empty(aa)).map_err(xe)?;

            w.write_event(Event::End(BytesEnd::new("xs:complexType"))).map_err(xe)?;
            w.write_event(Event::End(BytesEnd::new("xs:element"))).map_err(xe)?;
        }

        // ── Per-class declarations ─────────────────────────────────────────────
        let mut classes: Vec<&ClassDefinition> = dict
            .all_definitions()
            .filter_map(|d| {
                if let Definition::Class(c) = d {
                    Some(c)
                } else {
                    None
                }
            })
            .collect();
        classes.sort_by(|a, b| a.symbol.cmp(&b.symbol));

        for class in &classes {
            let members = self.resolver.get_members_of(class);
            let mut props: Vec<&PropertyDefinition> = members
                .iter()
                .filter_map(|id| {
                    self.resolver.get_definition(id).and_then(|d| {
                        if let Definition::Property(p) = d {
                            Some(p)
                        } else {
                            None
                        }
                    })
                })
                .collect();
            props.sort_by(|a, b| a.symbol.cmp(&b.symbol));

            write_class_type(&mut w, class, &props)?;
            write_class_element(&mut w, class)?;
        }

        // ── Close xs:schema ───────────────────────────────────────────────────
        w.write_event(Event::End(BytesEnd::new("xs:schema"))).map_err(xe)?;

        Ok(())
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn xe(e: std::io::Error) -> FragmentError {
    FragmentError::Xml(e.to_string())
}

/// Generate a named complexType for a class:
/// ```xml
/// <xs:complexType name="PrefaceType">
///   <xs:sequence>
///     <xs:element name="PropertyA" type="xs:anyType" minOccurs="0"/>
///     ...
///     <xs:any namespace="##other" minOccurs="0" maxOccurs="unbounded" processContents="lax"/>
///   </xs:sequence>
///   <xs:attribute ref="reg:uid"/>
///   <xs:anyAttribute namespace="##other" processContents="lax"/>
/// </xs:complexType>
/// ```
fn write_class_type<W: Write>(
    w: &mut Writer<W>,
    class: &ClassDefinition,
    props: &[&PropertyDefinition],
) -> Result<(), FragmentError> {
    let type_name = format!("{}Type", class.symbol);
    let mut ct = BytesStart::new("xs:complexType");
    ct.push_attribute(("name", type_name.as_str()));
    w.write_event(Event::Start(ct)).map_err(xe)?;

    // Optional annotation with source namespace
    if !class.namespace.is_empty() {
        w.write_event(Event::Start(BytesStart::new("xs:annotation"))).map_err(xe)?;
        w.write_event(Event::Start(BytesStart::new("xs:documentation"))).map_err(xe)?;
        w.write_event(Event::Text(BytesText::new(&class.namespace))).map_err(xe)?;
        w.write_event(Event::End(BytesEnd::new("xs:documentation"))).map_err(xe)?;
        w.write_event(Event::End(BytesEnd::new("xs:annotation"))).map_err(xe)?;
    }

    w.write_event(Event::Start(BytesStart::new("xs:sequence"))).map_err(xe)?;

    for prop in props {
        let mut pe = BytesStart::new("xs:element");
        pe.push_attribute(("name", prop.symbol.as_str()));
        pe.push_attribute(("type", "xs:anyType"));
        pe.push_attribute(("minOccurs", "0"));
        pe.push_attribute(("maxOccurs", "1"));
        w.write_event(Event::Empty(pe)).map_err(xe)?;
    }

    // Allow any additional elements (dark / extension properties)
    let mut any = BytesStart::new("xs:any");
    any.push_attribute(("namespace", "##other"));
    any.push_attribute(("minOccurs", "0"));
    any.push_attribute(("maxOccurs", "unbounded"));
    any.push_attribute(("processContents", "lax"));
    w.write_event(Event::Empty(any)).map_err(xe)?;

    w.write_event(Event::End(BytesEnd::new("xs:sequence"))).map_err(xe)?;

    // reg:uid attribute
    let mut uid_ref = BytesStart::new("xs:attribute");
    uid_ref.push_attribute(("ref", "reg:uid"));
    w.write_event(Event::Empty(uid_ref)).map_err(xe)?;

    // Open-ended attributes (reg:type, reg:byteOrder, etc.)
    let mut aa = BytesStart::new("xs:anyAttribute");
    aa.push_attribute(("namespace", "##other"));
    aa.push_attribute(("processContents", "lax"));
    w.write_event(Event::Empty(aa)).map_err(xe)?;

    w.write_event(Event::End(BytesEnd::new("xs:complexType"))).map_err(xe)?;
    Ok(())
}

/// Generate a top-level element declaration referencing the class type:
/// ```xml
/// <xs:element name="Preface" type="reg:PrefaceType"/>
/// ```
fn write_class_element<W: Write>(
    w: &mut Writer<W>,
    class: &ClassDefinition,
) -> Result<(), FragmentError> {
    let type_name = format!("reg:{}Type", class.symbol);
    let mut elem = BytesStart::new("xs:element");
    elem.push_attribute(("name", class.symbol.as_str()));
    elem.push_attribute(("type", type_name.as_str()));
    w.write_event(Event::Empty(elem)).map_err(xe)?;
    Ok(())
}

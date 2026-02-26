use std::{fs, path::PathBuf};

use regxml_dict::{
    definition::{Definition, TypeDefinition},
    importer::import_registers,
    meta_dictionary::{MetaDictionary, MetaDictionaryCollection},
    resolver::DefinitionResolver,
};
use smpte_types::Auid;

fn resources() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("resources")
}

fn read_resource(path: &str) -> Vec<u8> {
    fs::read(resources().join(path)).expect("resource not found")
}

// ─── MetaDictionary::from_xml ────────────────────────────────────────────────

#[test]
fn from_xml_parses_2003_dict() {
    let xml = read_resource("regxml-dicts/www-smpte-ra-org-reg-2003-2012.xml");
    let dict = MetaDictionary::from_xml(&xml).expect("parse failed");
    // Should have many definitions
    assert!(
        dict.definition_count() > 100,
        "expected many definitions, got {}",
        dict.definition_count()
    );
}

#[test]
fn from_xml_finds_integer_type() {
    let xml = read_resource("regxml-dicts/www-smpte-ra-org-reg-2003-2012.xml");
    let dict = MetaDictionary::from_xml(&xml).expect("parse failed");

    // UInt8 is one of the most basic types — UL 060e2b34.01040101.01010100.00000000
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.01040101.01010100.00000000").expect("bad urn");
    let def = dict.get_definition(&ul).expect("UInt8 not found");
    match def {
        Definition::Type(TypeDefinition::Integer(td)) => {
            assert_eq!(td.symbol, "UInt8");
            assert_eq!(td.size, 1);
            assert!(!td.is_signed);
        }
        other => panic!("expected Integer type, got {other:?}"),
    }
}

#[test]
fn from_xml_finds_class_definition() {
    // ClassDefinitions are in the 395-2014 metadictionary (Groups register).
    let xml = read_resource("regxml-dicts/www-smpte-ra-org-reg-395-2014.xml");
    let dict = MetaDictionary::from_xml(&xml).expect("parse failed");

    // DMCVTGenericSet1 — first class in that file
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.027f0101.05310101.00000000").expect("bad urn");
    match dict.get_definition(&ul) {
        Some(Definition::Class(cd)) => {
            assert_eq!(cd.symbol, "DMCVTGenericSet1");
            assert!(!cd.is_concrete);
        }
        other => panic!("expected ClassDefinition, got {other:?}"),
    }
}

#[test]
fn from_xml_symbol_lookup() {
    let xml = read_resource("regxml-dicts/www-smpte-ra-org-reg-2003-2012.xml");
    let dict = MetaDictionary::from_xml(&xml).expect("parse failed");
    let def = dict
        .get_definition_by_symbol("UInt16")
        .expect("UInt16 not found");
    assert!(matches!(def, Definition::Type(TypeDefinition::Integer(_))));
}

#[test]
fn from_xml_395_extension_parses() {
    let xml = read_resource("regxml-dicts/www-smpte-ra-org-reg-395-2014.xml");
    let dict = MetaDictionary::from_xml(&xml).expect("parse failed");
    // Should contain a few class definitions
    assert!(dict.definition_count() > 0);
}

#[test]
fn collection_merges_dicts() {
    let xml1 = read_resource("regxml-dicts/www-smpte-ra-org-reg-2003-2012.xml");
    let xml2 = read_resource("regxml-dicts/www-smpte-ra-org-reg-395-2014.xml");

    let collection =
        MetaDictionaryCollection::from_xml_slices(&[&xml1, &xml2]).expect("parse failed");

    // Both UInt8 (from 2003) and DMCVTApp1Set (from 395) should be findable.
    let ul8 = Auid::from_urn("urn:smpte:ul:060e2b34.01040101.01010100.00000000").unwrap();
    assert!(collection.get_definition(&ul8).is_some());

    let ul_dmcvt = Auid::from_urn("urn:smpte:ul:060e2b34.027f0101.05310201.00000000").unwrap();
    assert!(collection.get_definition(&ul_dmcvt).is_some());
}

// ─── importer::import_registers ──────────────────────────────────────────────

#[test]
fn import_types_register_integers() {
    let types_xml = read_resource("registers/Types.xml");
    let dict = import_registers(&[&types_xml]).expect("import failed");

    // UInt8 must come through
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.01040101.01010100.00000000").unwrap();
    let def = dict.get_definition(&ul).expect("UInt8 not found");
    match def {
        Definition::Type(TypeDefinition::Integer(td)) => {
            assert_eq!(td.symbol, "UInt8");
            assert_eq!(td.size, 1);
            assert!(!td.is_signed);
        }
        other => panic!("expected Integer, got {other:?}"),
    }
}

#[test]
fn import_types_register_signed_integer() {
    let types_xml = read_resource("registers/Types.xml");
    let dict = import_registers(&[&types_xml]).expect("import failed");

    // Int8 — UL 060e2b34.01040101.01010500.00000000
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.01040101.01010500.00000000").unwrap();
    let def = dict.get_definition(&ul).expect("Int8 not found");
    match def {
        Definition::Type(TypeDefinition::Integer(td)) => {
            assert!(td.is_signed, "Int8 should be signed");
        }
        other => panic!("expected Integer, got {other:?}"),
    }
}

#[test]
fn import_types_register_rename() {
    let types_xml = read_resource("registers/Types.xml");
    let dict = import_registers(&[&types_xml]).expect("import failed");

    // PositionType is a Rename of Int64
    let by_symbol = dict.get_definition_by_symbol("PositionType");
    assert!(by_symbol.is_some(), "PositionType not found");
    assert!(matches!(
        by_symbol.unwrap(),
        Definition::Type(TypeDefinition::Rename(_))
    ));
}

#[test]
fn import_types_register_strong_reference() {
    let types_xml = read_resource("registers/Types.xml");
    let dict = import_registers(&[&types_xml]).expect("import failed");

    // There should be many StrongReference types
    let strong_refs: Vec<_> = dict
        .all_definitions()
        .filter(|d| matches!(d, Definition::Type(TypeDefinition::StrongReference(_))))
        .collect();
    assert!(!strong_refs.is_empty(), "no StrongReference types found");
}

#[test]
fn import_types_register_enumeration() {
    let types_xml = read_resource("registers/Types.xml");
    let dict = import_registers(&[&types_xml]).expect("import failed");

    // Boolean — TypeKind=Enumeration with numeric values
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.01040101.01040100.00000000").unwrap();
    match dict.get_definition(&ul) {
        Some(Definition::Type(TypeDefinition::Enumeration(td))) => {
            assert_eq!(td.symbol, "Boolean");
            assert_eq!(td.elements.len(), 2);
            assert_eq!(td.elements[0].name, "False");
            assert_eq!(td.elements[0].value, 0);
        }
        other => panic!("expected Enumeration, got {other:?}"),
    }
}

#[test]
fn import_types_register_extendible_enumeration() {
    let types_xml = read_resource("registers/Types.xml");
    let dict = import_registers(&[&types_xml]).expect("import failed");

    // OperationCategoryType is ExtendibleEnumeration
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.01040101.02020101.00000000").unwrap();
    match dict.get_definition(&ul) {
        Some(Definition::Type(TypeDefinition::ExtendibleEnumeration(td))) => {
            assert_eq!(td.symbol, "OperationCategoryType");
        }
        other => panic!("expected ExtendibleEnumeration, got {other:?}"),
    }
}

#[test]
fn import_groups_and_elements_registers() {
    let types_xml = read_resource("registers/Types.xml");
    let elements_xml = read_resource("registers/Elements.xml");
    let groups_xml = read_resource("registers/Groups.xml");

    let dict = import_registers(&[&types_xml, &elements_xml, &groups_xml]).expect("import failed");

    // InterchangeObject class should be present
    let ul = Auid::from_urn("urn:smpte:ul:060e2b34.027f0101.0d010101.01010100").unwrap();
    match dict.get_definition(&ul) {
        Some(Definition::Class(cd)) => {
            assert_eq!(cd.symbol, "InterchangeObject");
        }
        other => panic!("expected ClassDefinition, got {other:?}"),
    }
}

#[test]
fn import_groups_produces_property_definitions() {
    let types_xml = read_resource("registers/Types.xml");
    let elements_xml = read_resource("registers/Elements.xml");
    let groups_xml = read_resource("registers/Groups.xml");

    let dict = import_registers(&[&types_xml, &elements_xml, &groups_xml]).expect("import failed");

    let props: Vec<_> = dict
        .all_definitions()
        .filter(|d| matches!(d, Definition::Property(_)))
        .collect();
    assert!(!props.is_empty(), "no property definitions produced");
}

#[test]
fn import_groups_parent_class() {
    let types_xml = read_resource("registers/Types.xml");
    let elements_xml = read_resource("registers/Elements.xml");
    let groups_xml = read_resource("registers/Groups.xml");

    let dict = import_registers(&[&types_xml, &elements_xml, &groups_xml]).expect("import failed");

    // StaticTrack has a parent class
    let def = dict.get_definition_by_symbol("StaticTrack");
    match def {
        Some(Definition::Class(cd)) => {
            assert!(
                cd.parent_class.is_some(),
                "StaticTrack should have a parent class"
            );
        }
        other => panic!("expected ClassDefinition, got {other:?}"),
    }
}

use smpte_types::Auid;

// ── Class Definition ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClassDefinition {
    pub identification: Auid,
    pub symbol: String,
    pub namespace: String,
    pub parent_class: Option<Auid>,
    pub is_concrete: bool,
}

// ── Property Definition ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PropertyDefinition {
    pub identification: Auid,
    pub symbol: String,
    pub namespace: String,
    pub member_of: Auid,
    pub property_type: Auid,
    pub is_optional: bool,
    pub is_unique_identifier: bool,
    pub local_identification: u16,
}

// ── Type Definitions ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CharacterTypeDef      { pub identification: Auid, pub symbol: String, pub namespace: String }
#[derive(Debug, Clone)]
pub struct EnumerationElement    { pub name: String, pub value: i64 }
#[derive(Debug, Clone)]
pub struct EnumerationTypeDef    { pub identification: Auid, pub symbol: String, pub namespace: String, pub element_type: Auid, pub elements: Vec<EnumerationElement> }
#[derive(Debug, Clone)]
pub struct ExtEnumTypeDef        { pub identification: Auid, pub symbol: String, pub namespace: String, pub elements: Vec<EnumerationElement> }
#[derive(Debug, Clone)]
pub struct FixedArrayTypeDef     { pub identification: Auid, pub symbol: String, pub namespace: String, pub element_type: Auid, pub element_count: u32 }
#[derive(Debug, Clone)]
pub struct FloatTypeDef          { pub identification: Auid, pub symbol: String, pub namespace: String, pub size: u8 }
#[derive(Debug, Clone)]
pub struct IndirectTypeDef       { pub identification: Auid, pub symbol: String, pub namespace: String }
#[derive(Debug, Clone)]
pub struct IntegerTypeDef        { pub identification: Auid, pub symbol: String, pub namespace: String, pub size: u8, pub is_signed: bool }
#[derive(Debug, Clone)]
pub struct LensSerialFloatTypeDef{ pub identification: Auid, pub symbol: String, pub namespace: String }
#[derive(Debug, Clone)]
pub struct OpaqueTypeDef         { pub identification: Auid, pub symbol: String, pub namespace: String }
#[derive(Debug, Clone)]
pub struct RecordMember          { pub name: String, pub field_type: Auid }
#[derive(Debug, Clone)]
pub struct RecordTypeDef         { pub identification: Auid, pub symbol: String, pub namespace: String, pub members: Vec<RecordMember> }
#[derive(Debug, Clone)]
pub struct RenameTypeDef         { pub identification: Auid, pub symbol: String, pub namespace: String, pub renamed_type: Auid }
#[derive(Debug, Clone)]
pub struct SetTypeDef            { pub identification: Auid, pub symbol: String, pub namespace: String, pub element_type: Auid }
#[derive(Debug, Clone)]
pub struct StreamTypeDef         { pub identification: Auid, pub symbol: String, pub namespace: String }
#[derive(Debug, Clone)]
pub struct StringTypeDef         { pub identification: Auid, pub symbol: String, pub namespace: String, pub element_type: Auid }
#[derive(Debug, Clone)]
pub struct StrongReferenceTypeDef{ pub identification: Auid, pub symbol: String, pub namespace: String, pub referenced_type: Auid }
#[derive(Debug, Clone)]
pub struct VariableArrayTypeDef  { pub identification: Auid, pub symbol: String, pub namespace: String, pub element_type: Auid }
#[derive(Debug, Clone)]
pub struct WeakReferenceTypeDef  { pub identification: Auid, pub symbol: String, pub namespace: String, pub referenced_type: Auid, pub target_set: Vec<Auid> }

/// All 17 SMPTE type variants (ST 2001-1 §11).
#[derive(Debug, Clone)]
pub enum TypeDefinition {
    Character(CharacterTypeDef),
    Enumeration(EnumerationTypeDef),
    ExtendibleEnumeration(ExtEnumTypeDef),
    FixedArray(FixedArrayTypeDef),
    Float(FloatTypeDef),
    Indirect(IndirectTypeDef),
    Integer(IntegerTypeDef),
    LensSerialFloat(LensSerialFloatTypeDef),
    Opaque(OpaqueTypeDef),
    Record(RecordTypeDef),
    Rename(RenameTypeDef),
    Set(SetTypeDef),
    Stream(StreamTypeDef),
    String(StringTypeDef),
    StrongReference(StrongReferenceTypeDef),
    VariableArray(VariableArrayTypeDef),
    WeakReference(WeakReferenceTypeDef),
}

// ── Top-level Definition ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Definition {
    Class(ClassDefinition),
    Property(PropertyDefinition),
    Type(TypeDefinition),
}

impl Definition {
    pub fn identification(&self) -> &Auid {
        match self {
            Definition::Class(d)    => &d.identification,
            Definition::Property(d) => &d.identification,
            Definition::Type(td)    => td.identification(),
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Definition::Class(d)    => &d.symbol,
            Definition::Property(d) => &d.symbol,
            Definition::Type(td)    => td.symbol(),
        }
    }
}

impl TypeDefinition {
    pub fn identification(&self) -> &Auid {
        match self {
            TypeDefinition::Character(d)             => &d.identification,
            TypeDefinition::Enumeration(d)           => &d.identification,
            TypeDefinition::ExtendibleEnumeration(d) => &d.identification,
            TypeDefinition::FixedArray(d)            => &d.identification,
            TypeDefinition::Float(d)                 => &d.identification,
            TypeDefinition::Indirect(d)              => &d.identification,
            TypeDefinition::Integer(d)               => &d.identification,
            TypeDefinition::LensSerialFloat(d)       => &d.identification,
            TypeDefinition::Opaque(d)                => &d.identification,
            TypeDefinition::Record(d)                => &d.identification,
            TypeDefinition::Rename(d)                => &d.identification,
            TypeDefinition::Set(d)                   => &d.identification,
            TypeDefinition::Stream(d)                => &d.identification,
            TypeDefinition::String(d)                => &d.identification,
            TypeDefinition::StrongReference(d)       => &d.identification,
            TypeDefinition::VariableArray(d)         => &d.identification,
            TypeDefinition::WeakReference(d)         => &d.identification,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            TypeDefinition::Character(d)             => &d.symbol,
            TypeDefinition::Enumeration(d)           => &d.symbol,
            TypeDefinition::ExtendibleEnumeration(d) => &d.symbol,
            TypeDefinition::FixedArray(d)            => &d.symbol,
            TypeDefinition::Float(d)                 => &d.symbol,
            TypeDefinition::Indirect(d)              => &d.symbol,
            TypeDefinition::Integer(d)               => &d.symbol,
            TypeDefinition::LensSerialFloat(d)       => &d.symbol,
            TypeDefinition::Opaque(d)                => &d.symbol,
            TypeDefinition::Record(d)                => &d.symbol,
            TypeDefinition::Rename(d)                => &d.symbol,
            TypeDefinition::Set(d)                   => &d.symbol,
            TypeDefinition::Stream(d)                => &d.symbol,
            TypeDefinition::String(d)                => &d.symbol,
            TypeDefinition::StrongReference(d)       => &d.symbol,
            TypeDefinition::VariableArray(d)         => &d.symbol,
            TypeDefinition::WeakReference(d)         => &d.symbol,
        }
    }

    pub fn namespace(&self) -> &str {
        match self {
            TypeDefinition::Character(d)             => &d.namespace,
            TypeDefinition::Enumeration(d)           => &d.namespace,
            TypeDefinition::ExtendibleEnumeration(d) => &d.namespace,
            TypeDefinition::FixedArray(d)            => &d.namespace,
            TypeDefinition::Float(d)                 => &d.namespace,
            TypeDefinition::Indirect(d)              => &d.namespace,
            TypeDefinition::Integer(d)               => &d.namespace,
            TypeDefinition::LensSerialFloat(d)       => &d.namespace,
            TypeDefinition::Opaque(d)                => &d.namespace,
            TypeDefinition::Record(d)                => &d.namespace,
            TypeDefinition::Rename(d)                => &d.namespace,
            TypeDefinition::Set(d)                   => &d.namespace,
            TypeDefinition::Stream(d)                => &d.namespace,
            TypeDefinition::String(d)                => &d.namespace,
            TypeDefinition::StrongReference(d)       => &d.namespace,
            TypeDefinition::VariableArray(d)         => &d.namespace,
            TypeDefinition::WeakReference(d)         => &d.namespace,
        }
    }
}

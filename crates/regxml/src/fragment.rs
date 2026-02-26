use std::collections::{HashMap, HashSet};
use std::io::Write;

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use regxml_dict::{
    definition::{ClassDefinition, Definition, PropertyDefinition, TypeDefinition},
    DefinitionResolver,
};
use smpte_klv::LocalSet;
use smpte_types::{Auid, Ul};

use crate::{
    event::{EventCode, EventHandlerDecision, EventSeverity, FragmentEvent},
    EventHandler, FragmentError,
};

// ── Namespace constants ───────────────────────────────────────────────────────

/// RegXML baseline namespace (SMPTE ST 2001-1).
pub const REGXML_NS: &str = "http://www.smpte-ra.org/schemas/2001-1b/2013/baseline";
pub const REG_PREFIX: &str = "reg";

// ── Special UL constants (byte arrays for compile-time use) ──────────────────
// Bit ordering for equals_with_mask: bit k ↔ byte (15-k), MSB-first.

// InstanceID property (SMPTE ST 335 §8.1): 060e2b34.01010101.01011502.00000000
const INSTANCE_UID_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x15, 0x02, 0x00, 0x00, 0x00, 0x00,
];

// ByteOrder property: 060e2b34.01010101.03010201.02000000
const BYTE_ORDER_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x01, 0x03, 0x01, 0x02, 0x01, 0x02, 0x00, 0x00, 0x00,
];

// PrimaryPackage property: 060e2b34.01010104.06010104.01080000
const PRIMARY_PACKAGE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x04, 0x06, 0x01, 0x01, 0x04, 0x01, 0x08, 0x00, 0x00,
];

// LinkedGenerationID property: 060e2b34.01010102.05200701.08000000
const LINKED_GENERATION_ID_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x02, 0x05, 0x20, 0x07, 0x01, 0x08, 0x00, 0x00, 0x00,
];

// GenerationID property: 060e2b34.01010102.05200701.01000000
const GENERATION_ID_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x02, 0x05, 0x20, 0x07, 0x01, 0x01, 0x00, 0x00, 0x00,
];

// ApplicationProductID property: 060e2b34.01010102.05200701.07000000
const APPLICATION_PRODUCT_ID_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x01, 0x01, 0x02, 0x05, 0x20, 0x07, 0x01, 0x07, 0x00, 0x00, 0x00,
];

// Type ULs used in Record special-casing (applyRule5_8):
// AUID type: 060e2b34.01040101.01030100.00000000
const AUID_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x01, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// UUID type: 060e2b34.01040101.01030300.00000000
const UUID_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x01, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// PackageIDType: 060e2b34.01040101.01030200.00000000
const PACKAGE_ID_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x01, 0x03, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Rational: 060e2b34.01040101.03010100.00000000
const RATIONAL_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x03, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// DateStruct: 060e2b34.01040101.03010500.00000000
const DATE_STRUCT_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x03, 0x01, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// TimeStruct: 060e2b34.01040101.03010600.00000000
const TIME_STRUCT_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x03, 0x01, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// TimeStamp: 060e2b34.01040101.03010700.00000000
const TIME_STAMP_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x03, 0x01, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// VersionType: 060e2b34.01040101.03010300.00000000
const VERSION_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x03, 0x01, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Character (UTF-16BE): 060e2b34.01040101.01100100.00000000
const CHARACTER_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x01, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Char (US-ASCII): 060e2b34.01040101.01100300.00000000
const CHAR_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x01, 0x10, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// UTF8Character: 060e2b34.01040101.01100500.00000000
#[allow(dead_code)]
const UTF8_CHARACTER_TYPE_UL: [u8; 16] = [
    0x06, 0x0E, 0x2B, 0x34, 0x01, 0x04, 0x01, 0x01, 0x01, 0x10, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// ── Optional callback for resolving an Auid to a human-readable symbol ────────

/// Optional callback for resolving an [`Auid`] to a human-readable symbol name.
pub trait AuidNamer: Send + Sync {
    fn name_of(&self, id: &Auid) -> Option<&str>;
}

// ── FragmentBuilder ───────────────────────────────────────────────────────────

/// Converts KLV local sets (MXF header metadata groups) into RegXML XML fragments
/// conforming to SMPTE ST 2001-1 Rules 3–5.
pub struct FragmentBuilder<'dict, R: DefinitionResolver> {
    resolver: &'dict R,
    /// Instance UUID → LocalSet for strong reference resolution.
    sets: HashMap<Auid, LocalSet>,
    handler: Option<Box<dyn EventHandler>>,
    auid_namer: Option<Box<dyn AuidNamer>>,
    /// namespace URI → prefix (e.g. REGXML_NS → "reg")
    ns_prefixes: HashMap<String, String>,
}

impl<'dict, R: DefinitionResolver> FragmentBuilder<'dict, R> {
    pub fn new(resolver: &'dict R) -> Self {
        let mut ns_prefixes = HashMap::new();
        ns_prefixes.insert(REGXML_NS.to_owned(), REG_PREFIX.to_owned());
        FragmentBuilder {
            resolver,
            sets: HashMap::new(),
            handler: None,
            auid_namer: None,
            ns_prefixes,
        }
    }

    pub fn with_event_handler(mut self, h: Box<dyn EventHandler>) -> Self {
        self.handler = Some(h);
        self
    }

    pub fn with_auid_namer(mut self, n: Box<dyn AuidNamer>) -> Self {
        self.auid_namer = Some(n);
        self
    }

    pub fn add_set(&mut self, id: Auid, set: LocalSet) {
        self.sets.insert(id, set);
    }

    /// Convert `group` to a RegXML XML fragment, writing to `writer`.
    pub fn from_triplet(
        &mut self,
        group: &LocalSet,
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        let mut visited = HashSet::new();
        self.apply_rule3(group, writer, &mut visited)
    }

    // ── namespace helpers ─────────────────────────────────────────────────────

    /// Return the prefix for `ns`, allocating a new one if needed.
    fn ns_prefix(&mut self, ns: &str) -> String {
        if let Some(p) = self.ns_prefixes.get(ns) {
            return p.clone();
        }
        let prefix = format!("ns{}", self.ns_prefixes.len());
        self.ns_prefixes.insert(ns.to_owned(), prefix.clone());
        prefix
    }

    // ── event helper ──────────────────────────────────────────────────────────

    fn fire_event(
        &mut self,
        code: EventCode,
        severity: EventSeverity,
        location: &str,
        reason: &str,
    ) -> Result<(), FragmentError> {
        let ev = FragmentEvent {
            code,
            severity,
            location: location.to_owned(),
            reason: reason.to_owned(),
        };
        if let Some(h) = &mut self.handler {
            if h.handle(&ev) == EventHandlerDecision::Abort {
                return Err(FragmentError::Fatal(reason.to_owned()));
            }
        }
        Ok(())
    }

    // ── Rule 3: Group → XML element ───────────────────────────────────────────

    fn apply_rule3(
        &mut self,
        group: &LocalSet,
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // 1. Resolve class definition.
        // MXF group keys use byte[5]=0x53 (two-byte local set), while the SMPTE
        // Groups register stores class ULs with byte[5]=0x7F (wildcard). Try both.
        let cd = match self.resolver.get_definition(&group.key).or_else(|| {
            normalize_class_key(&group.key)
                .as_ref()
                .and_then(|k| self.resolver.get_definition(k))
        }) {
            Some(Definition::Class(c)) => c.clone(),
            _ => {
                self.fire_event(
                    EventCode::UnknownGroup,
                    EventSeverity::Warn,
                    &group.key.to_string(),
                    "group key not found in dictionary",
                )?;
                return Ok(());
            }
        };

        // 2. Find InstanceUID for cycle detection (always INSTANCE_UID_UL key).
        // Use version-normalised comparison: byte 7 may differ between primer and constant.
        let iuid_norm = ul_zero_version(Auid::from_bytes(INSTANCE_UID_UL));
        let instance_uid: Option<Auid> = group
            .items
            .iter()
            .find(|item| ul_zero_version(item.key) == iuid_norm)
            .and_then(|item| {
                if item.value.len() == 16 {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(&item.value);
                    Some(Auid::from_bytes(b))
                } else {
                    None
                }
            });

        // 3. Circular strong-reference detection.
        if let Some(uid) = instance_uid {
            if visited.contains(&uid) {
                self.fire_event(
                    EventCode::CircularStrongReference,
                    EventSeverity::Warn,
                    &uid.to_string(),
                    "circular strong reference detected",
                )?;
                return Ok(());
            }
            visited.insert(uid);
        }

        // 5. Collect all inherited properties of this class.
        let all_props = get_all_members_of(self.resolver, &cd);

        // 6. Find the unique-identifier property value for reg:uid.
        // Per ST 2001-1 §7.2, reg:uid is the value of the property whose
        // PropertyDefinition has is_unique_identifier=true.  For most classes
        // (InterchangeObject → InstanceID is NOT flagged unique in the Groups
        // register) this is absent.  For Package subclasses the PackageID IS
        // flagged, giving a UMID. For EssenceData it gives a UMID too.
        let reg_uid_str: Option<String> = group.items.iter().find_map(|item| {
            let pd = all_props.get(&ul_zero_version(item.key))?;
            if !pd.is_unique_identifier {
                return None;
            }
            // Skip InstanceID itself — it is the fallback cycle-detection key,
            // not the user-facing canonical identifier.
            if ul_zero_version(item.key) == iuid_norm {
                return None;
            }
            match item.value.len() {
                32 => Some(format_umid_bytes(&item.value)),
                16 => Some(format_uuid_bytes(&item.value)),
                _ => None,
            }
        });

        // 7. Write start element.
        let ns = cd.namespace.clone();
        let prefix = self.ns_prefix(&ns);
        let elem_name = format!("{}:{}", prefix, cd.symbol);

        let mut start = BytesStart::new(elem_name.clone());

        // Declare reg: namespace on every group element (safe, idempotent in XML).
        start.push_attribute(("xmlns:reg", REGXML_NS));
        // Declare the class namespace if different from reg:
        if ns != REGXML_NS {
            start.push_attribute((format!("xmlns:{prefix}").as_str(), ns.as_str()));
        }

        // reg:uid from the class-specific unique-identifier property (if any).
        if let Some(ref uid_str) = reg_uid_str {
            start.push_attribute(("reg:uid", uid_str.as_str()));
        }

        writer
            .write_event(Event::Start(start))
            .map_err(|e| FragmentError::Xml(e.to_string()))?;

        // 7. Write property items.
        for item in &group.items {
            // Look up property using version-normalised key (byte 7 zeroed) so
            // that mismatches between the primer's version byte and the register's
            // version byte do not cause properties to be silently skipped.
            let normalised_key = ul_zero_version(item.key);
            if let Some(pd) = all_props.get(&normalised_key) {
                let pd = pd.clone();
                self.apply_rule4(&pd, &item.value, writer, visited)?;
            } else {
                self.fire_event(
                    EventCode::UnknownProperty,
                    EventSeverity::Warn,
                    &item.key.to_string(),
                    "property key not found in dictionary",
                )?;
            }
        }

        // 8. End element.
        writer
            .write_event(Event::End(BytesEnd::new(elem_name)))
            .map_err(|e| FragmentError::Xml(e.to_string()))?;

        // 9. Remove from visited set.
        if let Some(uid) = instance_uid {
            visited.remove(&uid);
        }

        Ok(())
    }

    // ── Rule 4: Property item ─────────────────────────────────────────────────

    fn apply_rule4(
        &mut self,
        pd: &PropertyDefinition,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // Special case: LinkedGenerationID / GenerationID / ApplicationProductID
        // are UUID-typed but should be emitted as urn:uuid: text nodes.
        let prop_ul = Auid::from_bytes(LINKED_GENERATION_ID_UL);
        let gen_ul = Auid::from_bytes(GENERATION_ID_UL);
        let app_ul = Auid::from_bytes(APPLICATION_PRODUCT_ID_UL);
        let prim_ul = Auid::from_bytes(PRIMARY_PACKAGE_UL);

        let is_uuid_override = pd.identification == prop_ul
            || pd.identification == gen_ul
            || pd.identification == app_ul;

        // Resolve type definition.
        let type_def = match self.resolver.get_definition(&pd.property_type) {
            Some(Definition::Type(td)) => td.clone(),
            _ => {
                // Unknown type — emit raw hex.
                let ns = pd.namespace.clone();
                let prefix = self.ns_prefix(&ns);
                let elem_name = format!("{}:{}", prefix, pd.symbol);
                self.write_simple_elem(writer, &elem_name, &hex_bytes(data))?;
                return Ok(());
            }
        };

        // Prepare element name.
        let ns = pd.namespace.clone();
        let prefix = self.ns_prefix(&ns);
        let elem_name = format!("{}:{}", prefix, pd.symbol);

        // Write start element.
        let mut start = BytesStart::new(elem_name.clone());
        if ns != REGXML_NS {
            start.push_attribute((format!("xmlns:{prefix}").as_str(), ns.as_str()));
        }
        // No reg:type attribute — not emitted per ST 2001-1 Java reference.
        // Special case: PrimaryPackage — handled via strong reference rule below.
        let _ = prim_ul;

        writer
            .write_event(Event::Start(start))
            .map_err(|e| FragmentError::Xml(e.to_string()))?;

        // Special case: ByteOrder — emit decoded "BigEndian"/"LittleEndian" string
        // instead of the raw Int16 value, matching Java reference behaviour.
        let bo_norm = ul_zero_version(Auid::from_bytes(BYTE_ORDER_UL));
        if is_uuid_override {
            // Emit 16-byte value as urn:uuid: regardless of declared type.
            let text = if data.len() == 16 {
                let mut b = [0u8; 16];
                b.copy_from_slice(data);
                format_uuid_bytes(&b)
            } else {
                hex_bytes(data)
            };
            writer
                .write_event(Event::Text(BytesText::new(&text)))
                .map_err(|e| FragmentError::Xml(e.to_string()))?;
        } else if ul_zero_version(pd.identification) == bo_norm {
            let text = decode_byte_order(data).unwrap_or_else(|| hex_bytes(data));
            writer
                .write_event(Event::Text(BytesText::new(&text)))
                .map_err(|e| FragmentError::Xml(e.to_string()))?;
        } else {
            self.apply_rule5(&type_def, data, writer, visited)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new(elem_name)))
            .map_err(|e| FragmentError::Xml(e.to_string()))?;

        Ok(())
    }

    // ── Rule 5: Type dispatch ─────────────────────────────────────────────────

    fn apply_rule5(
        &mut self,
        td: &TypeDefinition,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // Unwrap Rename chain first to get the base type.
        let base = find_base_definition(self.resolver, td);
        match &base {
            TypeDefinition::Integer(d) => self.rule5_integer(d.size, d.is_signed, data, writer),
            TypeDefinition::Character(d) => self.rule5_character(&d.identification, data, writer),
            TypeDefinition::String(d) => {
                let d = d.clone();
                self.rule5_string(&d, data, writer)
            }
            TypeDefinition::Record(d) => {
                let d = d.clone();
                self.rule5_record(&d, data, writer, visited)
            }
            TypeDefinition::Enumeration(d) => {
                let d = d.clone();
                self.rule5_enumeration(&d, data, writer)
            }
            TypeDefinition::ExtendibleEnumeration(d) => {
                let d = d.clone();
                self.rule5_ext_enumeration(&d, data, writer)
            }
            TypeDefinition::FixedArray(d) => {
                let d = d.clone();
                self.rule5_fixed_array(&d, data, writer, visited)
            }
            TypeDefinition::VariableArray(d) => {
                let d = d.clone();
                self.rule5_variable_array(&d, data, writer, visited)
            }
            TypeDefinition::Set(d) => {
                let d = d.clone();
                self.rule5_set(&d, data, writer, visited)
            }
            TypeDefinition::StrongReference(_) => self.rule5_strong_ref(data, writer, visited),
            TypeDefinition::WeakReference(d) => {
                let d = d.clone();
                self.rule5_weak_ref(&d, data, writer)
            }
            TypeDefinition::Float(d) => self.rule5_float(d.size, data, writer),
            TypeDefinition::LensSerialFloat(_) => self.rule5_lens_serial_float(data, writer),
            TypeDefinition::Indirect(_) => self.rule5_indirect(data, writer, visited),
            TypeDefinition::Opaque(_) => self.rule5_opaque(data, writer),
            TypeDefinition::Stream(_) => self.rule5_stream(data, writer),
            TypeDefinition::Rename(_) => {
                // Should not happen after find_base_definition, but handle gracefully.
                writer
                    .write_event(Event::Text(BytesText::new(&hex_bytes(data))))
                    .map_err(|e| FragmentError::Xml(e.to_string()))
            }
        }
    }

    // ── Rule 5.5: Integer ─────────────────────────────────────────────────────

    fn rule5_integer(
        &self,
        size: u8,
        is_signed: bool,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        let text = if is_signed {
            match (size, data.len()) {
                (1, 1) => (data[0] as i8).to_string(),
                (2, 2) => i16::from_be_bytes([data[0], data[1]]).to_string(),
                (4, 4) => i32::from_be_bytes([data[0], data[1], data[2], data[3]]).to_string(),
                (8, 8) => i64::from_be_bytes(data[..8].try_into().unwrap()).to_string(),
                _ => hex_bytes(data),
            }
        } else {
            match (size, data.len()) {
                (1, 1) => data[0].to_string(),
                (2, 2) => u16::from_be_bytes([data[0], data[1]]).to_string(),
                (4, 4) => u32::from_be_bytes([data[0], data[1], data[2], data[3]]).to_string(),
                (8, 8) => u64::from_be_bytes(data[..8].try_into().unwrap()).to_string(),
                _ => hex_bytes(data),
            }
        };
        writer
            .write_event(Event::Text(BytesText::new(&text)))
            .map_err(|e| FragmentError::Xml(e.to_string()))
    }

    // ── Rule 5.1: Character ───────────────────────────────────────────────────

    fn rule5_character(
        &self,
        type_id: &Auid,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        let text = if type_id.as_ul().map(Ul::as_bytes) == Some(&CHARACTER_TYPE_UL) {
            read_characters_utf16be(data, false)
        } else if type_id.as_ul().map(Ul::as_bytes) == Some(&CHAR_TYPE_UL) {
            read_characters_ascii(data, false)
        } else {
            // UTF8Character or unknown → UTF-8
            read_characters_utf8(data, false)
        };
        writer
            .write_event(Event::Text(BytesText::new(&text)))
            .map_err(|e| FragmentError::Xml(e.to_string()))
    }

    // ── Rule 5.12: String ─────────────────────────────────────────────────────

    fn rule5_string(
        &self,
        td: &regxml_dict::definition::StringTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // Determine encoding from the element_type (character type).
        let elem_ul = td.element_type.as_ul().map(Ul::as_bytes).copied();
        let text = match elem_ul {
            Some(b) if b == CHAR_TYPE_UL => read_characters_ascii(data, true),
            Some(b) if b == UTF8_CHARACTER_TYPE_UL => read_characters_utf8(data, true),
            // Default: UTF-16BE (covers CHARACTER_TYPE_UL and unknown)
            _ => read_characters_utf16be(data, true),
        };
        writer
            .write_event(Event::Text(BytesText::new(&text)))
            .map_err(|e| FragmentError::Xml(e.to_string()))
    }

    // ── Rule 5.8: Record ──────────────────────────────────────────────────────

    fn rule5_record(
        &mut self,
        td: &regxml_dict::definition::RecordTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        let id_bytes = match td.identification.as_ul().map(Ul::as_bytes) {
            Some(b) => *b,
            None => return self.rule5_record_general(td, data, writer, visited),
        };

        match id_bytes {
            AUID_TYPE_UL => self.rule5_record_auid(data, writer),
            UUID_TYPE_UL => self.rule5_record_uuid(data, writer),
            PACKAGE_ID_TYPE_UL => self.rule5_record_package_id(data, writer),
            RATIONAL_TYPE_UL => self.rule5_record_rational(data, writer),
            DATE_STRUCT_TYPE_UL => self.rule5_record_date(data, writer),
            TIME_STRUCT_TYPE_UL => self.rule5_record_time(data, writer),
            TIME_STAMP_TYPE_UL => self.rule5_record_timestamp(data, writer),
            VERSION_TYPE_UL => self.rule5_record_version(data, writer),
            _ => self.rule5_record_general(td, data, writer, visited),
        }
    }

    fn rule5_record_auid(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        if data.len() < 16 {
            return write_text(writer, &hex_bytes(data));
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(&data[..16]);
        let auid = Auid::from_bytes(b);
        write_text(writer, &auid.to_string())
    }

    fn rule5_record_uuid(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        if data.len() < 16 {
            return write_text(writer, &hex_bytes(data));
        }
        let text = format_uuid_bytes(&data[..16]);
        write_text(writer, &text)
    }

    fn rule5_record_package_id(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // UMID is 32 bytes; format as hex groups (SMPTE ST 330).
        if data.len() < 32 {
            return write_text(writer, &hex_bytes(data));
        }
        // Format as urn:smpte:umid:XXXXXXXX.XXXXXXXX.XXXXXXXX.XXXXXXXX.XXXXXXXX.XXXXXXXX.XXXXXXXX.XXXXXXXX
        let hex: String = data[..32]
            .chunks(4)
            .map(|chunk| {
                format!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    chunk[0], chunk[1], chunk[2], chunk[3]
                )
            })
            .collect::<Vec<_>>()
            .join(".");
        write_text(writer, &format!("urn:smpte:umid:{hex}"))
    }

    fn rule5_record_rational(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        if data.len() < 8 {
            return write_text(writer, &hex_bytes(data));
        }
        let num = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let den = i32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        write_text(writer, &format!("{num}/{den}"))
    }

    fn rule5_record_date(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // DateStruct: year(u16) + month(u8) + day(u8)
        if data.len() < 4 {
            return write_text(writer, &hex_bytes(data));
        }
        let year = u16::from_be_bytes([data[0], data[1]]);
        let month = data[2];
        let day = data[3];
        write_text(writer, &format!("{year:04}-{month:02}-{day:02}"))
    }

    fn rule5_record_time(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // TimeStruct: hour(u8) + min(u8) + sec(u8) + frm(u8)
        if data.len() < 4 {
            return write_text(writer, &hex_bytes(data));
        }
        write_text(
            writer,
            &format!(
                "{:02}:{:02}:{:02}.{:02}",
                data[0], data[1], data[2], data[3]
            ),
        )
    }

    fn rule5_record_timestamp(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // TimeStamp: DateStruct(4) + TimeStruct(4)
        // TimeStruct fraction is in 1/250 second units → convert to milliseconds.
        if data.len() < 8 {
            return write_text(writer, &hex_bytes(data));
        }
        let year = u16::from_be_bytes([data[0], data[1]]);
        let month = data[2];
        let day = data[3];
        let frac_ms = data[7] as u32 * 4; // 1/250 s → ms (×4)
        let ts = if frac_ms == 0 {
            format!(
                "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
                data[4], data[5], data[6]
            )
        } else {
            format!(
                "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
                data[4], data[5], data[6], frac_ms
            )
        };
        write_text(writer, &ts)
    }

    fn rule5_record_version(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // VersionType: major(u8) + minor(u8)
        if data.len() < 2 {
            return write_text(writer, &hex_bytes(data));
        }
        write_text(writer, &format!("{}.{}", data[0], data[1]))
    }

    fn rule5_record_general(
        &mut self,
        td: &regxml_dict::definition::RecordTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // Record fields share the record type's namespace (not reg: baseline).
        let field_ns = td.namespace.clone();
        let members = td.members.clone();

        // Walk each field in order, consuming bytes by field type size.
        let mut cursor = 0usize;
        for (i, member) in members.iter().enumerate() {
            let is_last = i == members.len() - 1;

            // Resolve field type to determine its size.
            let field_td = match self.resolver.get_definition(&member.field_type) {
                Some(Definition::Type(t)) => t.clone(),
                _ => break,
            };
            let size = type_size_bytes(&field_td, self.resolver);
            let slice = if is_last {
                // Last field: consume all remaining bytes. This handles cases where
                // encoders write more bytes than the type definition implies (e.g.
                // ProductReleaseType stored as 2 bytes despite UInt8 base type).
                if cursor >= data.len() {
                    break;
                }
                &data[cursor..]
            } else if let Some(s) = size {
                if cursor + s > data.len() {
                    break;
                }
                let sl = &data[cursor..cursor + s];
                cursor += s;
                sl
            } else {
                // Variable-size non-last field: consume remaining bytes and stop.
                let sl = &data[cursor..];
                cursor = data.len();
                sl
            };

            let field_prefix = self.ns_prefix(&field_ns);
            let elem_name = format!("{}:{}", field_prefix, member.name);
            let mut start = BytesStart::new(elem_name.clone());
            if field_ns != REGXML_NS {
                start.push_attribute((format!("xmlns:{field_prefix}").as_str(), field_ns.as_str()));
            }
            writer
                .write_event(Event::Start(start))
                .map_err(|e| FragmentError::Xml(e.to_string()))?;
            self.apply_rule5(&field_td, slice, writer, visited)?;
            writer
                .write_event(Event::End(BytesEnd::new(elem_name)))
                .map_err(|e| FragmentError::Xml(e.to_string()))?;

            if !is_last && size.is_none() {
                break;
            }
        }
        Ok(())
    }

    // ── Rule 5.2: Enumeration ─────────────────────────────────────────────────

    fn rule5_enumeration(
        &self,
        td: &regxml_dict::definition::EnumerationTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // Read integer value.
        let val: i64 = match data.len() {
            1 => data[0] as i8 as i64,
            2 => i16::from_be_bytes([data[0], data[1]]) as i64,
            4 => i32::from_be_bytes(data[..4].try_into().unwrap()) as i64,
            8 => i64::from_be_bytes(data[..8].try_into().unwrap()),
            _ => return write_text(writer, &hex_bytes(data)),
        };
        let name = td
            .elements
            .iter()
            .find(|e| e.value == val)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| val.to_string());
        write_text(writer, &name)
    }

    // ── Rule 5.2a: ExtendibleEnumeration ─────────────────────────────────────

    fn rule5_ext_enumeration(
        &self,
        td: &regxml_dict::definition::ExtEnumTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // ExtEnum value is a 16-byte UL.
        if data.len() < 16 {
            return write_text(writer, &hex_bytes(data));
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(&data[..16]);
        let auid = Auid::from_bytes(b);
        // 1. Check inline elements (rarely populated from register XMLs alone).
        let auid_str = auid.to_string();
        if let Some(e) = td
            .elements
            .iter()
            .find(|e| e.name == auid_str || e.value.to_string() == auid_str)
        {
            return write_text(writer, &e.name);
        }
        // 2. Try the AuidNamer (populated from Labels.xml when provided).
        if let Some(name) = self.auid_namer.as_ref().and_then(|n| n.name_of(&auid)) {
            return write_text(writer, name);
        }
        // 3. Fall back to the AUID URN string.
        write_text(writer, &auid_str)
    }

    // ── Rule 5.4: FixedArray ──────────────────────────────────────────────────

    fn rule5_fixed_array(
        &mut self,
        td: &regxml_dict::definition::FixedArrayTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // UUID FixedArray special case: render as urn:uuid: string.
        if td.identification.as_ul().map(Ul::as_bytes) == Some(&UUID_TYPE_UL) {
            return self.rule5_record_uuid(data, writer);
        }

        let count = td.element_count as usize;
        if count == 0 || data.is_empty() {
            return Ok(());
        }
        let elem_size = data.len() / count;
        let elem_td = match self.resolver.get_definition(&td.element_type) {
            Some(Definition::Type(t)) => t.clone(),
            _ => return write_text(writer, &hex_bytes(data)),
        };

        // StrongReference array: resolve each element as a strong ref.
        let base_elem = find_base_definition(self.resolver, &elem_td);
        if matches!(base_elem, TypeDefinition::StrongReference(_)) {
            for i in 0..count {
                let slice = &data[i * elem_size..(i + 1) * elem_size];
                self.rule5_strong_ref(slice, writer, visited)?;
            }
            return Ok(());
        }

        // Use the element type's symbol+namespace for the item element name.
        let item_ns = elem_td.namespace().to_owned();
        let item_sym = elem_td.symbol().to_owned();
        let item_prefix = self.ns_prefix(&item_ns);
        let item_tag = format!("{}:{}", item_prefix, item_sym);

        for i in 0..count {
            let slice = &data[i * elem_size..(i + 1) * elem_size];
            let mut item_start = BytesStart::new(item_tag.clone());
            if item_ns != REGXML_NS {
                item_start
                    .push_attribute((format!("xmlns:{item_prefix}").as_str(), item_ns.as_str()));
            }
            writer
                .write_event(Event::Start(item_start))
                .map_err(|e| FragmentError::Xml(e.to_string()))?;
            self.apply_rule5(&elem_td, slice, writer, visited)?;
            writer
                .write_event(Event::End(BytesEnd::new(item_tag.as_str())))
                .map_err(|e| FragmentError::Xml(e.to_string()))?;
        }
        Ok(())
    }

    // ── Rule 5.14: VariableArray ──────────────────────────────────────────────

    fn rule5_variable_array(
        &mut self,
        td: &regxml_dict::definition::VariableArrayTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // VariableArray<UInt8 or Int8>: data is stored as raw bytes (no batch-
        // format header). Emit as compact hex (no separators) matching Java output.
        if let Some(Definition::Type(elem_td)) = self.resolver.get_definition(&td.element_type) {
            let base = find_base_definition(self.resolver, elem_td);
            if let TypeDefinition::Integer(int_def) = &base {
                if int_def.size == 1 {
                    return write_text(writer, &hex_bytes_compact(data));
                }
            }
        }
        self.rule5_batch_or_array(&td.element_type, data, writer, visited)
    }

    // ── Rule 5.10: Set/Batch ──────────────────────────────────────────────────

    fn rule5_set(
        &mut self,
        td: &regxml_dict::definition::SetTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        self.rule5_batch_or_array(&td.element_type, data, writer, visited)
    }

    /// Shared batch format parser: u32 count + u32 item_len + N items.
    fn rule5_batch_or_array(
        &mut self,
        elem_type_id: &Auid,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        if data.len() < 8 {
            return write_text(writer, &hex_bytes(data));
        }
        let count = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let item_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let body = &data[8..];

        if item_len == 0 {
            return Ok(());
        }

        let elem_td = match self.resolver.get_definition(elem_type_id) {
            Some(Definition::Type(t)) => t.clone(),
            _ => return write_text(writer, &hex_bytes(data)),
        };

        // StrongReference elements: write referenced objects directly, no item wrapper.
        let base_elem = find_base_definition(self.resolver, &elem_td);
        let is_strong_ref = matches!(base_elem, TypeDefinition::StrongReference(_));

        // For non-strong-ref items, use the element type's symbol+namespace.
        let item_ns = elem_td.namespace().to_owned();
        let item_sym = elem_td.symbol().to_owned();
        let item_prefix = self.ns_prefix(&item_ns);
        let item_tag = format!("{}:{}", item_prefix, item_sym);

        for i in 0..count {
            let start = i * item_len;
            let end = start + item_len;
            if end > body.len() {
                break;
            }
            let slice = &body[start..end];
            if is_strong_ref {
                self.apply_rule5(&elem_td, slice, writer, visited)?;
            } else {
                let mut item_start = BytesStart::new(item_tag.clone());
                if item_ns != REGXML_NS {
                    item_start.push_attribute((
                        format!("xmlns:{item_prefix}").as_str(),
                        item_ns.as_str(),
                    ));
                }
                writer
                    .write_event(Event::Start(item_start))
                    .map_err(|e| FragmentError::Xml(e.to_string()))?;
                self.apply_rule5(&elem_td, slice, writer, visited)?;
                writer
                    .write_event(Event::End(BytesEnd::new(item_tag.as_str())))
                    .map_err(|e| FragmentError::Xml(e.to_string()))?;
            }
        }
        Ok(())
    }

    // ── Rule 5.13: StrongReference ────────────────────────────────────────────

    fn rule5_strong_ref(
        &mut self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        if data.len() < 16 {
            return write_text(writer, &hex_bytes(data));
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(&data[..16]);
        let uid = Auid::from_bytes(b);

        // Clone the target set to avoid borrow conflict with &mut self.
        let target = match self.sets.get(&uid) {
            Some(s) => s.clone(),
            None => {
                // Target not found — emit as urn string.
                return write_text(writer, &uid.to_string());
            }
        };
        self.apply_rule3(&target, writer, visited)
    }

    // ── Rule 5.15: WeakReference ──────────────────────────────────────────────

    fn rule5_weak_ref(
        &self,
        _td: &regxml_dict::definition::WeakReferenceTypeDef,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // A WeakReference stores the InstanceUID (16 bytes) of the target object.
        // Per ST 2001-1, emit the target's unique-identifier property value (e.g.,
        // PackageID/UMID for Package subclasses), not the raw InstanceUID.
        if data.len() != 16 {
            return write_text(writer, &hex_bytes(data));
        }
        let mut b = [0u8; 16];
        b.copy_from_slice(data);
        let lookup_uid = Auid::from_bytes(b);

        // Try to resolve to the target's unique identifier.
        if let Some(target_set) = self.sets.get(&lookup_uid) {
            let target_set = target_set.clone();
            // Find the class definition for the target.
            let cd = match self.resolver.get_definition(&target_set.key).or_else(|| {
                normalize_class_key(&target_set.key)
                    .as_ref()
                    .and_then(|k| self.resolver.get_definition(k))
            }) {
                Some(Definition::Class(c)) => c.clone(),
                _ => return write_text(writer, &lookup_uid.to_string()),
            };
            let all_props = get_all_members_of(self.resolver, &cd);
            let iuid_norm = ul_zero_version(Auid::from_bytes(INSTANCE_UID_UL));
            // Find the unique-identifier property (not InstanceID).
            let uid_str = target_set.items.iter().find_map(|item| {
                let pd = all_props.get(&ul_zero_version(item.key))?;
                if !pd.is_unique_identifier {
                    return None;
                }
                if ul_zero_version(item.key) == iuid_norm {
                    return None;
                }
                match item.value.len() {
                    32 => Some(format_umid_bytes(&item.value)),
                    16 => {
                        let mut arr = [0u8; 16];
                        arr.copy_from_slice(&item.value);
                        // Use Auid::from_bytes: UL-shaped bytes (bit7=0) → urn:smpte:ul:…
                        Some(Auid::from_bytes(arr).to_string())
                    }
                    _ => None,
                }
            });
            if let Some(s) = uid_str {
                return write_text(writer, &s);
            }
        }

        // Fallback: use Auid::from_bytes so UL-shaped bytes (bit7=0) format as
        // urn:smpte:ul:… and true UUIDs format as urn:uuid:….
        let auid = Auid::from_bytes(b);
        write_text(writer, &auid.to_string())
    }

    // ── Rule 5.alpha: Float ───────────────────────────────────────────────────

    fn rule5_float(
        &self,
        size: u8,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        let text = match (size, data.len()) {
            (2, 2) => {
                let bits = u16::from_be_bytes([data[0], data[1]]);
                let f = half::f16::from_bits(bits);
                format!("{}", f.to_f64())
            }
            (4, 4) => {
                let v = f32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                format!("{v}")
            }
            (8, 8) => {
                let v = f64::from_be_bytes(data[..8].try_into().unwrap());
                format!("{v}")
            }
            _ => hex_bytes(data),
        };
        write_text(writer, &text)
    }

    // ── Rule 5.beta: LensSerialFloat ──────────────────────────────────────────

    fn rule5_lens_serial_float(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        // Canon lens serial number float: 32-bit special encoding.
        // Emit as hex for now (format is not publicly documented).
        write_text(writer, &hex_bytes(data))
    }

    // ── Rule 5.6: Indirect ────────────────────────────────────────────────────

    fn rule5_indirect(
        &mut self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
        visited: &mut HashSet<Auid>,
    ) -> Result<(), FragmentError> {
        // Layout: 1 byte byte-order + 16 byte type identifier + value bytes.
        // Byte-order byte: 0x42 ('B') = big-endian, 0x4C ('L') = little-endian.
        if data.len() < 17 {
            return write_text(writer, &hex_bytes(data));
        }
        let byte_order = data[0];
        let value = &data[17..];

        // Resolve type AUID, applying UUID LE byte-swap + half-swap when little-endian.
        // AAF/MXF indirect type bytes are stored as a UUID with byte-order-specific
        // encoding: for LE, the first three UUID fields are individually reversed, then
        // the two 8-byte halves are swapped to recover the standard SMPTE UL byte order.
        let b = &data[1..17];
        let type_bytes: [u8; 16] = if byte_order == 0x4C {
            // UUID-LE swap (reverse groups 0-3, 4-5, 6-7, keep 8-15) then half-swap.
            // Combined: output[0..8] = b[8..16], output[8..12] = b[3..0], etc.
            [
                b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15], b[3], b[2], b[1], b[0], b[5],
                b[4], b[7], b[6],
            ]
        } else {
            // Big-endian: bytes are already in standard UL/UUID format.
            let mut arr = [0u8; 16];
            arr.copy_from_slice(b);
            arr
        };
        let type_auid = Auid::from_bytes(type_bytes);

        let type_def = match self.resolver.get_definition(&type_auid) {
            Some(Definition::Type(td)) => td.clone(),
            _ => return write_text(writer, &hex_bytes(value)),
        };

        // For string types with little-endian byte order, decode as UTF-16LE.
        if byte_order == 0x4C {
            let base = find_base_definition(self.resolver, &type_def);
            if matches!(base, TypeDefinition::String(_)) {
                let text = read_characters_utf16le(value, true);
                return write_text(writer, &text);
            }
        }

        self.apply_rule5(&type_def, value, writer, visited)
    }

    // ── Rule 5.7: Opaque ──────────────────────────────────────────────────────

    fn rule5_opaque(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        write_text(writer, &hex_bytes(data))
    }

    // ── Rule 5.11: Stream ─────────────────────────────────────────────────────

    fn rule5_stream(
        &self,
        data: &[u8],
        writer: &mut Writer<impl Write>,
    ) -> Result<(), FragmentError> {
        write_text(writer, &hex_bytes(data))
    }

    // ── Utility: write a simple tag with text content ─────────────────────────

    fn write_simple_elem(
        &self,
        writer: &mut Writer<impl Write>,
        tag: &str,
        text: &str,
    ) -> Result<(), FragmentError> {
        writer
            .write_event(Event::Start(BytesStart::new(tag)))
            .map_err(|e| FragmentError::Xml(e.to_string()))?;
        writer
            .write_event(Event::Text(BytesText::new(text)))
            .map_err(|e| FragmentError::Xml(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new(tag)))
            .map_err(|e| FragmentError::Xml(e.to_string()))
    }
}

// ── Free helper functions ─────────────────────────────────────────────────────

/// Recursively collect all properties of a class including inherited ones.
///
/// The returned map is keyed by **version-normalised** property AUIDs (byte 7
/// of the UL zeroed) so that lookups from the MXF primer succeed even when
/// the primer uses a different version byte than the SMPTE register.
fn get_all_members_of<R: DefinitionResolver>(
    resolver: &R,
    cd: &ClassDefinition,
) -> HashMap<Auid, PropertyDefinition> {
    let mut result = HashMap::new();
    // Parent class members first (so child overrides parent).
    if let Some(parent_id) = cd.parent_class {
        if let Some(Definition::Class(parent)) = resolver.get_definition(&parent_id) {
            let parent = parent.clone();
            result.extend(get_all_members_of(resolver, &parent));
        }
    }
    // Own members — keyed by version-normalised AUID.
    for member_id in resolver.get_members_of(cd) {
        if let Some(Definition::Property(pd)) = resolver.get_definition(&member_id) {
            result.insert(ul_zero_version(member_id), pd.clone());
        }
    }
    result
}

/// Normalise a property AUID for version-agnostic lookup.
///
/// SMPTE UL byte 7 is the version/revision byte and differs between the value
/// stored in the MXF primer and the value in the SMPTE register. Zero it out
/// so that `HashMap` lookups succeed regardless of which version byte is used.
fn ul_zero_version(auid: Auid) -> Auid {
    match auid {
        Auid::Ul(ul) => {
            let mut bytes = *ul.as_bytes();
            bytes[7] = 0;
            Auid::Ul(Ul::from_bytes(bytes))
        }
        other => other,
    }
}

/// Unwrap a Rename chain to reach the base TypeDefinition.
fn find_base_definition<R: DefinitionResolver>(
    resolver: &R,
    td: &TypeDefinition,
) -> TypeDefinition {
    let mut current = td.clone();
    // Guard against infinite loops.
    for _ in 0..32 {
        if let TypeDefinition::Rename(r) = &current {
            match resolver.get_definition(&r.renamed_type) {
                Some(Definition::Type(base)) => current = base.clone(),
                _ => break,
            }
        } else {
            break;
        }
    }
    current
}

/// Return the fixed byte-size of a type definition, or None for variable-size.
fn type_size_bytes<R: DefinitionResolver>(td: &TypeDefinition, resolver: &R) -> Option<usize> {
    match td {
        TypeDefinition::Integer(d) => Some(d.size as usize),
        TypeDefinition::Float(d) => Some(d.size as usize),
        TypeDefinition::Character(_) => Some(2), // UTF-16 char
        TypeDefinition::StrongReference(_) => Some(16),
        TypeDefinition::WeakReference(_) => Some(16),
        TypeDefinition::Rename(r) => {
            let base = match resolver.get_definition(&r.renamed_type) {
                Some(Definition::Type(t)) => t.clone(),
                _ => return None,
            };
            type_size_bytes(&base, resolver)
        }
        TypeDefinition::Record(r) => {
            let total: Option<usize> = r.members.iter().try_fold(0usize, |acc, m| {
                let member_td = resolver.get_definition(&m.field_type)?;
                if let Definition::Type(t) = member_td {
                    let s = type_size_bytes(t, resolver)?;
                    Some(acc + s)
                } else {
                    None
                }
            });
            total
        }
        TypeDefinition::FixedArray(fa) => {
            let elem_td = match resolver.get_definition(&fa.element_type) {
                Some(Definition::Type(t)) => t.clone(),
                _ => return None,
            };
            type_size_bytes(&elem_td, resolver).map(|s| s * fa.element_count as usize)
        }
        TypeDefinition::Enumeration(e) => {
            let base_td = match resolver.get_definition(&e.element_type) {
                Some(Definition::Type(t)) => t.clone(),
                _ => return None,
            };
            type_size_bytes(&base_td, resolver)
        }
        _ => None,
    }
}

/// Decode a ByteOrder property value (u16 big-endian) to "BigEndian" / "LittleEndian".
fn decode_byte_order(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let v = u16::from_be_bytes([data[0], data[1]]);
    match v {
        0x4D4D => Some("BigEndian".to_owned()),    // "MM"
        0x4949 => Some("LittleEndian".to_owned()), // "II"
        _ => None,
    }
}

/// Format 16 raw bytes as a `urn:uuid:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` string.
fn format_uuid_bytes(b: &[u8]) -> String {
    format!(
        "urn:uuid:{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    )
}

/// Format 32 raw bytes as a `urn:smpte:umid:XXXXXXXX.XXXXXXXX.…` string.
fn format_umid_bytes(b: &[u8]) -> String {
    let hex: String = b[..32]
        .chunks(4)
        .map(|chunk| {
            format!(
                "{:02x}{:02x}{:02x}{:02x}",
                chunk[0], chunk[1], chunk[2], chunk[3]
            )
        })
        .collect::<Vec<_>>()
        .join(".");
    format!("urn:smpte:umid:{hex}")
}

/// Normalize a group key for dictionary lookup.
///
/// MXF local set group keys encode the payload type in byte[5] (e.g. `0x53` for
/// two-byte tag + two-byte length). The SMPTE Groups register uses `0x7F`
/// (unspecified) for the same byte. When the exact AUID lookup fails, try with
/// byte[5] set to `0x7F` to match the registry form.
fn normalize_class_key(auid: &Auid) -> Option<Auid> {
    let ul = auid.as_ul()?;
    let mut bytes = *ul.as_bytes();
    // Only normalize if it looks like a group key (byte[4] == 0x02).
    if bytes[4] != 0x02 {
        return None;
    }
    if bytes[5] == 0x7F {
        return None; // already canonical
    }
    bytes[5] = 0x7F;
    Some(Auid::Ul(Ul::from_bytes(bytes)))
}

/// Hex-encode all bytes with space separators (used for unknown/opaque values).
fn hex_bytes(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Hex-encode all bytes without separators (used for raw byte arrays such as DataValue).
fn hex_bytes_compact(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

/// Apply ST 2001-1 §8.2.4 escape: C0/C1 control chars → `$#xH;`
///
/// The hex digits are lowercase with no leading zeros, matching the Java
/// reference implementation (e.g. `$#x1;` for U+0001, not `$#x0001;`).
fn st2001_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        let c = ch as u32;
        if (c < 0x20 && c != 0x09 && c != 0x0A && c != 0x0D)
            || (0x7F..=0x9F).contains(&c)
            || c == 0x00
        {
            out.push_str(&format!("$#x{c:x};"));
        } else {
            out.push(ch);
        }
    }
    out
}

/// Read UTF-16BE bytes as a string, optionally stripping null terminators.
fn read_characters_utf16be(data: &[u8], strip_nul: bool) -> String {
    if data.len() % 2 != 0 {
        return hex_bytes(data);
    }
    let (decoded, _, had_errors) = encoding_rs::UTF_16BE.decode(data);
    if had_errors {
        return hex_bytes(data);
    }
    let s = if strip_nul {
        decoded.trim_end_matches('\0').to_owned()
    } else {
        decoded.into_owned()
    };
    st2001_escape(&s)
}

fn read_characters_utf16le(data: &[u8], strip_nul: bool) -> String {
    if data.len() % 2 != 0 {
        return hex_bytes(data);
    }
    let (decoded, _, had_errors) = encoding_rs::UTF_16LE.decode(data);
    if had_errors {
        return hex_bytes(data);
    }
    let s = if strip_nul {
        decoded.trim_end_matches('\0').to_owned()
    } else {
        decoded.into_owned()
    };
    st2001_escape(&s)
}

/// Read US-ASCII bytes as a string.
fn read_characters_ascii(data: &[u8], strip_nul: bool) -> String {
    let s: String = data
        .iter()
        .take_while(|&&b| !strip_nul || b != 0)
        .map(|&b| if b < 0x80 { b as char } else { '?' })
        .collect();
    st2001_escape(&s)
}

/// Read UTF-8 bytes as a string.
fn read_characters_utf8(data: &[u8], strip_nul: bool) -> String {
    let s = std::str::from_utf8(data).unwrap_or("");
    let s = if strip_nul {
        s.trim_end_matches('\0')
    } else {
        s
    };
    st2001_escape(s)
}

/// Write a text node to the writer.
fn write_text(writer: &mut Writer<impl Write>, text: &str) -> Result<(), FragmentError> {
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(|e| FragmentError::Xml(e.to_string()))
}

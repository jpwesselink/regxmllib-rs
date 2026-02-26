//! SMPTE LabelsRegister parser.
//!
//! Parses a SMPTE Labels register XML (namespace `http://www.smpte-ra.org/schemas/400/2012`)
//! and provides UL → Symbol lookup.  Used by `regxml` to resolve ExtendibleEnumeration values.

use std::collections::HashMap;

use smpte_types::Auid;

use crate::DictError;

/// Parsed SMPTE LabelsRegister: maps 16-byte UL AUID → Symbol string.
pub struct LabelsRegister {
    map: HashMap<Auid, String>,
}

impl LabelsRegister {
    /// Parse a LabelsRegister XML byte slice.
    ///
    /// Returns an empty register (no error) if the root element is not
    /// `LabelsRegister`, so callers can pass any register XML without
    /// type-checking it first.
    pub fn from_xml(xml: &[u8]) -> Result<Self, DictError> {
        let text = std::str::from_utf8(xml).map_err(|e| DictError::Xml(e.to_string()))?;
        let doc = roxmltree::Document::parse(text).map_err(|e| DictError::Xml(e.to_string()))?;
        let root = doc.root_element();
        if root.tag_name().name() != "LabelsRegister" {
            return Ok(Self { map: HashMap::new() });
        }

        let mut map = HashMap::new();

        let entries = match root.children().find(|n| n.tag_name().name() == "Entries") {
            Some(e) => e,
            None => return Ok(Self { map }),
        };

        for entry in entries.children().filter(|n| n.tag_name().name() == "Entry") {
            // Skip NODE (non-leaf) entries.
            let kind = entry
                .children()
                .find(|n| n.tag_name().name() == "Kind")
                .and_then(|n| n.text());
            if kind == Some("NODE") {
                continue;
            }

            let ul_str = match entry
                .children()
                .find(|n| n.tag_name().name() == "UL")
                .and_then(|n| n.text())
            {
                Some(s) => s,
                None => continue,
            };

            let symbol = match entry
                .children()
                .find(|n| n.tag_name().name() == "Symbol")
                .and_then(|n| n.text())
            {
                Some(s) => s,
                None => continue,
            };

            if let Ok(auid) = Auid::from_urn(ul_str) {
                map.insert(auid, symbol.to_owned());
            }
        }

        Ok(Self { map })
    }

    /// Construct an empty register.
    pub fn empty() -> Self {
        Self { map: HashMap::new() }
    }

    /// Look up the symbol for a UL.
    pub fn get(&self, id: &Auid) -> Option<&str> {
        self.map.get(id).map(String::as_str)
    }

    /// Merge `other` into `self`, returning `self`.
    pub fn merge(mut self, other: Self) -> Self {
        for (k, v) in other.map {
            self.map.entry(k).or_insert(v);
        }
        self
    }

    /// Number of entries in the register.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the register contains no entries.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

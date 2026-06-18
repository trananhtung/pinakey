//! IBus D-Bus serializable types — ported from goibus `text.go`.
//!
//! Every IBus serializable object is a struct starting with `(s a{sv} ...)`. `IBusText` has
//! signature `(sa{sv}sv)`; `IBusAttrList` is `(sa{sv}av)`; `IBusAttribute` is `(sa{sv}uuuu)`.
//! The typed structs let zvariant derive the exact signatures (which also sidesteps the
//! empty-array signature problem), and the derived `Value`/`OwnedValue` conversions let us emit
//! them as D-Bus variants (`v`).

use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::constants::{IBUS_ATTR_TYPE_UNDERLINE, IBUS_ATTR_UNDERLINE_SINGLE};

#[derive(Type, serde::Serialize, serde::Deserialize, Value, OwnedValue)]
pub struct IBusAttribute {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub atype: u32,
    pub value: u32,
    pub start_index: u32,
    pub end_index: u32,
}

impl IBusAttribute {
    pub fn new(atype: u32, value: u32, start_index: u32, end_index: u32) -> Self {
        IBusAttribute {
            name: "IBusAttribute".to_string(),
            attachments: HashMap::new(),
            atype,
            value,
            start_index,
            end_index,
        }
    }
}

#[derive(Type, serde::Serialize, serde::Deserialize, Value, OwnedValue)]
pub struct IBusAttrList {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub attributes: Vec<OwnedValue>,
}

impl IBusAttrList {
    pub fn empty() -> Self {
        IBusAttrList {
            name: "IBusAttrList".to_string(),
            attachments: HashMap::new(),
            attributes: Vec::new(),
        }
    }
}

#[derive(Type, serde::Serialize, serde::Deserialize, Value, OwnedValue)]
pub struct IBusText {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub text: String,
    pub attr_list: OwnedValue,
}

impl IBusText {
    pub fn new(text: &str) -> zbus::zvariant::Result<Self> {
        let attr_list = OwnedValue::try_from(Value::from(IBusAttrList::empty()))?;
        Ok(IBusText {
            name: "IBusText".to_string(),
            attachments: HashMap::new(),
            text: text.to_string(),
            attr_list,
        })
    }

    /// Build text with a single underline attribute spanning `[0, len)` (matches `AppendAttr`).
    pub fn with_underline(text: &str, len: u32) -> zbus::zvariant::Result<Self> {
        let attr = IBusAttribute::new(IBUS_ATTR_TYPE_UNDERLINE, IBUS_ATTR_UNDERLINE_SINGLE, 0, len);
        let attr_list = IBusAttrList {
            name: "IBusAttrList".to_string(),
            attachments: HashMap::new(),
            attributes: vec![OwnedValue::try_from(Value::from(attr))?],
        };
        Ok(IBusText {
            name: "IBusText".to_string(),
            attachments: HashMap::new(),
            text: text.to_string(),
            attr_list: OwnedValue::try_from(Value::from(attr_list))?,
        })
    }

    /// Convert into a D-Bus variant value for emission in a signal.
    pub fn into_value(self) -> zbus::zvariant::Result<OwnedValue> {
        OwnedValue::try_from(Value::from(self))
    }
}

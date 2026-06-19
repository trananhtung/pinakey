//! Các kiểu dữ liệu D-Bus của IBus có thể serialize — chuyển thể từ `text.go` của goibus.
//!
//! Mọi đối tượng IBus serialize được đều là struct bắt đầu bằng `(s a{sv} ...)`. `IBusText` có
//! signature `(sa{sv}sv)`; `IBusAttrList` là `(sa{sv}av)`; `IBusAttribute` là `(sa{sv}uuuu)`.
//! Các struct có kiểu rõ ràng giúp zvariant suy ra đúng signature (đồng thời tránh được vấn đề
//! signature của mảng rỗng), và các phép chuyển đổi `Value`/`OwnedValue` được derive cho phép ta
//! phát chúng dưới dạng D-Bus variant (`v`).

use std::collections::HashMap;
use zbus::zvariant::{OwnedValue, Type, Value};

use crate::constants::{IBUS_ATTR_TYPE_UNDERLINE, IBUS_ATTR_UNDERLINE_SINGLE};
use crate::props::{Prop, PropKind};

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

    /// Tạo văn bản với một thuộc tính gạch chân duy nhất trải dài `[0, len)` (khớp `AppendAttr`).
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

    /// Chuyển thành giá trị D-Bus variant để phát trong một signal.
    pub fn into_value(self) -> zbus::zvariant::Result<OwnedValue> {
        OwnedValue::try_from(Value::from(self))
    }
}

// PropType của IBus.
const PROP_TYPE_NORMAL: u32 = 0;
const PROP_TYPE_TOGGLE: u32 = 1;
const PROP_TYPE_RADIO: u32 = 2;
// PropState của IBus.
const PROP_STATE_UNCHECKED: u32 = 0;
const PROP_STATE_CHECKED: u32 = 1;

/// Một mục menu thuộc tính của IBus — signature `(sa{sv}suvsvbbuv)`: key, type, label(IBusText),
/// icon, tooltip(IBusText), sensitive, visible, state, sub_props(IBusPropList).
#[derive(Type, serde::Serialize, serde::Deserialize, Value, OwnedValue)]
pub struct IBusProperty {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub key: String,
    pub prop_type: u32,
    pub label: OwnedValue,
    pub icon: String,
    pub tooltip: OwnedValue,
    pub sensitive: bool,
    pub visible: bool,
    pub state: u32,
    pub sub_props: OwnedValue,
}

impl IBusProperty {
    pub fn from_prop(p: &Prop) -> zbus::zvariant::Result<Self> {
        let prop_type = match p.kind {
            PropKind::Toggle => PROP_TYPE_TOGGLE,
            PropKind::Radio => PROP_TYPE_RADIO,
            PropKind::Action => PROP_TYPE_NORMAL,
        };
        let state = if p.checked {
            PROP_STATE_CHECKED
        } else {
            PROP_STATE_UNCHECKED
        };
        Ok(IBusProperty {
            name: "IBusProperty".to_string(),
            attachments: HashMap::new(),
            key: p.key.clone(),
            prop_type,
            label: IBusText::new(&p.label)?.into_value()?,
            icon: String::new(),
            tooltip: IBusText::new("")?.into_value()?,
            sensitive: true,
            visible: true,
            state,
            sub_props: IBusPropList::empty()?.into_value()?,
        })
    }

    pub fn into_value(self) -> zbus::zvariant::Result<OwnedValue> {
        OwnedValue::try_from(Value::from(self))
    }
}

/// Danh sách thuộc tính của IBus — signature `(sa{sv}av)`: mảng `IBusProperty`.
#[derive(Type, serde::Serialize, serde::Deserialize, Value, OwnedValue)]
pub struct IBusPropList {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub properties: Vec<OwnedValue>,
}

impl IBusPropList {
    pub fn empty() -> zbus::zvariant::Result<Self> {
        Ok(IBusPropList {
            name: "IBusPropList".to_string(),
            attachments: HashMap::new(),
            properties: Vec::new(),
        })
    }

    pub fn from_props(props: &[Prop]) -> zbus::zvariant::Result<Self> {
        let mut properties = Vec::with_capacity(props.len());
        for p in props {
            properties.push(IBusProperty::from_prop(p)?.into_value()?);
        }
        Ok(IBusPropList {
            name: "IBusPropList".to_string(),
            attachments: HashMap::new(),
            properties,
        })
    }

    pub fn into_value(self) -> zbus::zvariant::Result<OwnedValue> {
        OwnedValue::try_from(Value::from(self))
    }
}

/// Bảng tra cứu IBus — signature `(sa{sv}uubbiavav)`: page_size, cursor_pos, cursor_visible,
/// round, orientation, mảng ứng viên (av) và mảng nhãn (av), mỗi phần tử là một `IBusText`.
#[derive(Type, serde::Serialize, serde::Deserialize, Value, OwnedValue)]
pub struct IBusLookupTable {
    pub name: String,
    pub attachments: HashMap<String, OwnedValue>,
    pub page_size: u32,
    pub cursor_pos: u32,
    pub cursor_visible: bool,
    pub round: bool,
    pub orientation: i32,
    pub candidates: Vec<OwnedValue>,
    pub labels: Vec<OwnedValue>,
}

impl IBusLookupTable {
    /// Dựng bảng tra cứu từ danh sách ứng viên, với nhãn số `1.`…`9.` cho mỗi trang.
    pub fn new(
        candidates: &[String],
        cursor_pos: u32,
        page_size: u32,
    ) -> zbus::zvariant::Result<Self> {
        let mut cand_vals = Vec::with_capacity(candidates.len());
        for c in candidates {
            cand_vals.push(IBusText::new(c)?.into_value()?);
        }
        let mut labels = Vec::with_capacity(page_size as usize);
        for i in 0..page_size {
            labels.push(IBusText::new(&format!("{}.", i + 1))?.into_value()?);
        }
        Ok(IBusLookupTable {
            name: "IBusLookupTable".to_string(),
            attachments: HashMap::new(),
            page_size,
            cursor_pos,
            cursor_visible: true,
            round: true,
            orientation: 1, // dọc (vertical)
            candidates: cand_vals,
            labels,
        })
    }

    pub fn into_value(self) -> zbus::zvariant::Result<OwnedValue> {
        OwnedValue::try_from(Value::from(self))
    }
}

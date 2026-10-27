//! Between a place's native field values and a setting's neutral value, driven by the row's
//! [`Codec`]. Pure: no system is touched here, so planning can ask what a new system can hold
//! ([`quantize`]) before anything changes.

use pctwin_recipe::SettingValue;

use crate::catalogue::{Catalogue, Codec, Field, Kind, Place, Row, json_to_setting};
use crate::value::{ValueType, file_uri, path_of_file_uri, plain_name};

/// A field's value as the system holds it. `None` beside it (in a `Vec<Option<Native>>`) means
/// the field is not set.
#[derive(Debug, Clone, PartialEq)]
pub enum Native {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    List(Vec<String>),
}

/// The neutral value the native fields mean; `Ok(None)` when nothing is set (the system's
/// default applies). `Err` (plain words) when the native value is not one the row knows.
pub fn decode(
    cat: &Catalogue,
    row: &Row,
    place: &Place,
    natives: &[Option<Native>],
) -> Result<Option<SettingValue>, String> {
    let Some(codec) = place.codec() else {
        return Err("this setting has no way to read it".into());
    };
    let first = natives.first().cloned().flatten();
    let unknown = || "a value PCTwin does not know".to_string();
    let v = match codec {
        Codec::Same => match first {
            None => return Ok(None),
            Some(Native::Text(t)) if t.is_empty() => return Ok(None),
            Some(n) => same_to_neutral(&row.value, n).ok_or_else(unknown)?,
        },
        Codec::Flag { mask } => match first {
            None => return Ok(None),
            Some(Native::Int(i)) => SettingValue::Bool(i & mask != 0),
            Some(_) => return Err(unknown()),
        },
        Codec::Table { rows } => {
            let hit = rows.iter().find(|r| {
                r.native.len() == natives.len()
                    && r.native
                        .iter()
                        .zip(natives)
                        .all(|(m, n)| matches(m, n.as_ref()))
            });
            match hit {
                Some(r) => json_to_setting(&r.neutral).ok_or_else(unknown)?,
                None if natives.iter().all(Option::is_none) => return Ok(None),
                None => return Err(unknown()),
            }
        }
        Codec::Linear {
            native, neutral, ..
        } => {
            let x = match first {
                None => return Ok(None),
                Some(Native::Int(i)) => i as f64,
                Some(Native::Float(f)) => f,
                Some(_) => return Err(unknown()),
            };
            let (lo, hi) = (native[0].min(native[1]), native[0].max(native[1]));
            if !(lo..=hi).contains(&x) {
                return Err("a value outside what the system allows".into());
            }
            let n = neutral[0] as f64
                + (x - native[0]) * (neutral[1] - neutral[0]) as f64 / (native[1] - native[0]);
            SettingValue::Int(n.round() as i64)
        }
        Codec::FileUri => match first {
            None => return Ok(None),
            Some(Native::Text(t)) if t.is_empty() => return Ok(None),
            Some(Native::Text(t)) => SettingValue::Text(path_of_file_uri(&t).ok_or_else(unknown)?),
            Some(_) => return Err(unknown()),
        },
        Codec::Lookup {
            table,
            column,
            prefix,
        } => match first {
            None => return Ok(None),
            Some(Native::Text(t)) if t.is_empty() => return Ok(None),
            Some(Native::Text(t)) => {
                let tab = cat.tables.get(table).ok_or_else(unknown)?;
                SettingValue::Text(
                    tab.lookup(column, &t, "neutral", *prefix)
                        .ok_or_else(|| "not on PCTwin's list yet".to_string())?
                        .to_string(),
                )
            }
            Some(_) => return Err(unknown()),
        },
        Codec::ListLookup { table, column } => match first {
            None => return Ok(None),
            Some(Native::List(items)) => {
                let tab = cat.tables.get(table).ok_or_else(unknown)?;
                let mut out: Vec<String> = Vec::new();
                for item in items {
                    let n = match tab.lookup(column, &item, "neutral", false) {
                        Some(n) => n.to_string(),
                        None => {
                            let own = format!("{column}:{item}");
                            if !plain_name(&own) {
                                return Err(unknown());
                            }
                            own
                        }
                    };
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
                SettingValue::Text(out.join(","))
            }
            Some(_) => return Err(unknown()),
        },
        Codec::SplitFirst { sep } => match first {
            None => return Ok(None),
            Some(Native::Text(t)) => {
                let name = t
                    .split(sep.as_str())
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if name.is_empty() {
                    return Ok(None);
                }
                SettingValue::Text(name)
            }
            Some(_) => return Err(unknown()),
        },
    };
    row.value
        .check(row.id.as_str(), &v)
        .map_err(|e| e.to_string())?;
    Ok(Some(v))
}

/// The native field values that hold `value` (`None` for a field to remove). `Err` when this
/// place cannot hold it at all.
pub fn encode(
    cat: &Catalogue,
    row: &Row,
    place: &Place,
    value: &SettingValue,
) -> Result<Vec<Option<Native>>, String> {
    row.value
        .check(row.id.as_str(), value)
        .map_err(|e| e.to_string())?;
    let fields = place.fields();
    let cannot = || "the new laptop's system cannot hold this value".to_string();
    let codec = place.codec().ok_or_else(cannot)?;
    Ok(match codec {
        Codec::Same => vec![Some(same_to_native(&fields[0], value).ok_or_else(cannot)?)],
        Codec::Table { rows } => {
            let r = rows
                .iter()
                .find(|r| json_to_setting(&r.neutral).as_ref() == Some(value))
                .ok_or_else(cannot)?;
            let lits = r.write.as_ref().unwrap_or(&r.native);
            lits.iter()
                .zip(fields)
                .map(|(l, f)| literal_to_native(f, l).ok_or_else(cannot))
                .collect::<Result<_, _>>()?
        }
        Codec::Linear {
            native,
            neutral,
            decimals,
        } => {
            let SettingValue::Int(v) = value else {
                return Err(cannot());
            };
            let x = native[0]
                + (*v - neutral[0]) as f64 * (native[1] - native[0])
                    / (neutral[1] - neutral[0]) as f64;
            let n = match fields[0].kind {
                Kind::Float => {
                    let p = 10f64.powi(i32::from((*decimals).min(6)));
                    Native::Float((x * p).round() / p)
                }
                _ => Native::Int(x.round() as i64),
            };
            vec![Some(n)]
        }
        Codec::FileUri => {
            let SettingValue::Text(p) = value else {
                return Err(cannot());
            };
            vec![Some(Native::Text(file_uri(p).ok_or_else(cannot)?))]
        }
        Codec::Lookup { table, column, .. } => {
            let SettingValue::Text(v) = value else {
                return Err(cannot());
            };
            let tab = cat.tables.get(table).ok_or_else(cannot)?;
            vec![Some(Native::Text(
                tab.lookup("neutral", v, column, false)
                    .ok_or_else(cannot)?
                    .to_string(),
            ))]
        }
        Codec::ListLookup { table, column } => {
            let SettingValue::Text(v) = value else {
                return Err(cannot());
            };
            let tab = cat.tables.get(table).ok_or_else(cannot)?;
            let mut out = Vec::new();
            for item in v.split(',').filter(|i| !i.is_empty()) {
                let native = match item.split_once(':') {
                    Some((col, id)) if col == column => id.to_string(),
                    Some(_) => return Err(cannot()),
                    None => tab
                        .lookup("neutral", item, column, false)
                        .ok_or_else(cannot)?
                        .to_string(),
                };
                out.push(native);
            }
            vec![Some(Native::List(out))]
        }
        Codec::SplitFirst { .. } | Codec::Flag { .. } => return Err(cannot()),
    })
}

/// What `value` becomes on this place once written and read back: the nearest value the
/// system holds exactly. `Err` when it cannot hold it.
pub fn quantize(
    cat: &Catalogue,
    row: &Row,
    place: &Place,
    value: &SettingValue,
) -> Result<SettingValue, String> {
    let natives = encode(cat, row, place, value)?;
    decode(cat, row, place, &natives)?.ok_or_else(|| "nothing would be set".into())
}

fn same_to_neutral(t: &ValueType, n: Native) -> Option<SettingValue> {
    Some(match (t, n) {
        (ValueType::Int { .. }, Native::Int(i)) => SettingValue::Int(i),
        (ValueType::Bool, Native::Bool(b)) => SettingValue::Bool(b),
        (ValueType::Bool, Native::Int(i @ (0 | 1))) => SettingValue::Bool(i == 1),
        (ValueType::List { .. }, Native::List(l)) => SettingValue::Text(l.join(",")),
        (
            ValueType::Choice { .. }
            | ValueType::Name { .. }
            | ValueType::FilePath
            | ValueType::List { .. },
            Native::Text(s),
        ) => SettingValue::Text(s),
        _ => return None,
    })
}

fn same_to_native(f: &Field, v: &SettingValue) -> Option<Native> {
    Some(match (f.kind, v) {
        (Kind::Dword | Kind::Int | Kind::SzInt, SettingValue::Int(i)) => Native::Int(*i),
        (Kind::Float, SettingValue::Int(i)) => Native::Float(*i as f64),
        (Kind::Bool, SettingValue::Bool(b)) => Native::Bool(*b),
        (Kind::Dword | Kind::Int, SettingValue::Bool(b)) => Native::Int(i64::from(*b)),
        (Kind::Sz | Kind::String, SettingValue::Text(t)) => Native::Text(t.clone()),
        (k, SettingValue::Text(t)) if k.is_list() => Native::List(
            t.split(',')
                .filter(|i| !i.is_empty())
                .map(str::to_string)
                .collect(),
        ),
        _ => return None,
    })
}

/// Whether a table's match value fits a native field value.
fn matches(m: &serde_json::Value, n: Option<&Native>) -> bool {
    use serde_json::Value as J;
    match (m, n) {
        (J::Null, None) => true,
        (J::Object(_), Some(_)) => true, // `{"any": true}`, checked when the catalogue was read
        (J::Bool(b), Some(Native::Bool(x))) => b == x,
        (J::Number(j), Some(Native::Int(x))) => j.as_i64() == Some(*x),
        (J::Number(j), Some(Native::Float(x))) => j.as_f64() == Some(*x),
        (J::String(s), Some(Native::Text(x))) => s == x,
        // A registry text field holding a number, matched against a number or its text.
        (J::String(s), Some(Native::Int(x))) => s.parse::<i64>().ok() == Some(*x),
        _ => false,
    }
}

fn literal_to_native(f: &Field, l: &serde_json::Value) -> Option<Option<Native>> {
    use serde_json::Value as J;
    Some(match (f.kind, l) {
        (_, J::Null) => None,
        (Kind::Bool, J::Bool(b)) => Some(Native::Bool(*b)),
        (Kind::Dword | Kind::Int | Kind::SzInt, J::Number(n)) => Some(Native::Int(n.as_i64()?)),
        (Kind::SzInt, J::String(s)) => Some(Native::Int(s.parse().ok()?)),
        (Kind::Float, J::Number(n)) => Some(Native::Float(n.as_f64()?)),
        (Kind::Sz | Kind::String, J::String(s)) => Some(Native::Text(s.clone())),
        _ => return None,
    })
}

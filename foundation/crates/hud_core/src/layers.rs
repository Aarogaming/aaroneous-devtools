//! Layer merging with provenance and locks. Pure: layers arrive as text, nothing is read from disk.

use crate::error::LoadError;
use std::collections::BTreeMap;
use toml::{Table, Value};

/// Settings layers from lowest to highest precedence. `Defaults` is embedded and never supplied by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LayerKind {
    Defaults,
    User,
    Project,
    Local,
}

impl LayerKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Defaults => "defaults",
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
        }
    }
}

/// One caller-supplied layer. `text: None` means the file does not exist, which is normal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub kind: LayerKind,
    pub text: Option<String>,
}

impl Layer {
    pub fn new(kind: LayerKind, text: Option<&str>) -> Self {
        Self {
            kind,
            text: text.map(str::to_owned),
        }
    }
}

pub(crate) const RULES_PATH: &str = "permissions.rules";

pub(crate) struct Merged {
    pub table: Table,
    /// Dotted path of every non-table value to the layer that last set it.
    pub provenance: BTreeMap<String, LayerKind>,
    /// Rule texts in layer order (lowest first), each with the layer that wrote it.
    pub rules: Vec<(LayerKind, String)>,
}

fn leaves(table: &Table, prefix: &str, out: &mut Vec<String>) {
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Table(inner) => leaves(inner, &path, out),
            _ => out.push(path),
        }
    }
}

fn overlaps(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}.")) || b.starts_with(&format!("{a}."))
}

fn merge_into(
    base: &mut Table,
    overlay: Table,
    prefix: &str,
    kind: LayerKind,
    provenance: &mut BTreeMap<String, LayerKind>,
) {
    for (key, value) in overlay {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::Table(inner) => {
                let slot = base
                    .entry(key)
                    .or_insert_with(|| Value::Table(Table::new()));
                if let Value::Table(existing) = slot {
                    merge_into(existing, inner, &path, kind, provenance);
                } else {
                    // A scalar is being replaced by a table: forget the scalar's provenance.
                    provenance.remove(&path);
                    let mut fresh = Table::new();
                    merge_into(&mut fresh, inner, &path, kind, provenance);
                    *slot = Value::Table(fresh);
                }
            }
            other => {
                let prefix_dot = format!("{path}.");
                provenance.retain(|existing, _| !existing.starts_with(&prefix_dot));
                provenance.insert(path, kind);
                base.insert(key, other);
            }
        }
    }
}

fn take_lock_list(table: &mut Table, layer: &str) -> Result<Vec<String>, LoadError> {
    let Some(value) = table.remove("locked") else {
        return Ok(Vec::new());
    };
    let bad = || LoadError::BadLockList {
        layer: layer.to_owned(),
    };
    let Value::Array(items) = value else {
        return Err(bad());
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::String(path) if !path.is_empty() => Ok(path),
            _ => Err(bad()),
        })
        .collect()
}

fn take_rules(table: &mut Table, layer: &str) -> Result<Vec<String>, LoadError> {
    let Some(Value::Table(permissions)) = table.get_mut("permissions") else {
        return Ok(Vec::new());
    };
    let Some(value) = permissions.remove("rules") else {
        return Ok(Vec::new());
    };
    let invalid = || LoadError::Invalid {
        message: format!("{layer} settings: `{RULES_PATH}` must be an array of strings"),
    };
    let Value::Array(items) = value else {
        return Err(invalid());
    };
    items
        .into_iter()
        .map(|item| match item {
            Value::String(rule) => Ok(rule),
            _ => Err(invalid()),
        })
        .collect()
}

/// Merge layers in order. Locks set by a layer bind only the layers above it.
pub(crate) fn merge(layers: &[(LayerKind, &str)]) -> Result<Merged, LoadError> {
    let mut merged = Table::new();
    let mut provenance = BTreeMap::new();
    let mut rules = Vec::new();
    let mut locks: Vec<(String, LayerKind)> = Vec::new();
    for (kind, text) in layers {
        let mut table: Table = text
            .parse()
            .map_err(|error: toml::de::Error| LoadError::Parse {
                layer: kind.name().to_owned(),
                message: error.message().to_owned(),
            })?;
        let own_locks = take_lock_list(&mut table, kind.name())?;
        let own_rules = take_rules(&mut table, kind.name())?;

        let mut touched = Vec::new();
        leaves(&table, "", &mut touched);
        if !own_rules.is_empty() {
            touched.push(RULES_PATH.to_owned());
        }
        for path in &touched {
            if let Some((locked_path, locker)) =
                locks.iter().find(|(locked, _)| overlaps(path, locked))
            {
                return Err(LoadError::Locked {
                    path: locked_path.clone(),
                    locked_by: locker.name().to_owned(),
                    attempted_by: kind.name().to_owned(),
                });
            }
        }

        merge_into(&mut merged, table, "", *kind, &mut provenance);
        rules.extend(own_rules.into_iter().map(|rule| (*kind, rule)));
        locks.extend(own_locks.into_iter().map(|path| (path, *kind)));
    }
    Ok(Merged {
        table: merged,
        provenance,
        rules,
    })
}

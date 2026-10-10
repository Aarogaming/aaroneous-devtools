//! Load errors. Every variant names the layer or setting at fault so a person can fix the file.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// A layer's text is not valid TOML.
    Parse { layer: String, message: String },
    /// A higher layer set a key (or a parent or child of it) that a lower layer locked.
    Locked {
        path: String,
        locked_by: String,
        attempted_by: String,
    },
    /// `locked` is not an array of dotted-path strings.
    BadLockList { layer: String },
    /// A permission rule does not parse.
    BadRule {
        layer: String,
        rule: String,
        message: String,
    },
    /// The merged settings do not fit the schema or fail a range or consistency check.
    Invalid { message: String },
    /// The layers were not given as user, project, local in that order, each at most once.
    LayerOrder { message: String },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { layer, message } => {
                write!(f, "{layer} settings are not valid TOML: {message}")
            }
            Self::Locked {
                path,
                locked_by,
                attempted_by,
            } => write!(
                f,
                "{attempted_by} settings set `{path}`, which the {locked_by} settings locked"
            ),
            Self::BadLockList { layer } => {
                write!(
                    f,
                    "{layer} settings: `locked` must be an array of dotted setting paths"
                )
            }
            Self::BadRule {
                layer,
                rule,
                message,
            } => {
                write!(f, "{layer} settings: rule {rule:?}: {message}")
            }
            Self::Invalid { message } => write!(f, "invalid settings: {message}"),
            Self::LayerOrder { message } => write!(f, "settings layers: {message}"),
        }
    }
}

impl std::error::Error for LoadError {}

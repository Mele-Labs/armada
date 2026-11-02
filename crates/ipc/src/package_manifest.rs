//! What Fleet reads of a `package.json` to find the stylesheets of a package. This crate is where
//! bytes become values, so the manifest is read here and nowhere else.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

/// The parts of a manifest the stylesheet walk uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageManifest {
    pub name: Option<String>,
    /// The `style` field, the convention for a package's stylesheet.
    pub style: Option<String>,
    pub main: Option<String>,
    /// Every `.css` path in `exports`, in key order.
    pub css_exports: Vec<String>,
    /// The names it depends on, whatever the kind of dependency.
    pub depends_on: Vec<String>,
}

#[derive(Deserialize, Default)]
struct Raw {
    name: Option<Value>,
    style: Option<Value>,
    main: Option<Value>,
    exports: Option<Value>,
    #[serde(default)]
    dependencies: BTreeMap<String, Value>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: BTreeMap<String, Value>,
    #[serde(default, rename = "peerDependencies")]
    peer_dependencies: BTreeMap<String, Value>,
}

fn text(value: Option<Value>) -> Option<String> {
    match value {
        Some(Value::String(said)) if !said.trim().is_empty() => Some(said),
        _ => None,
    }
}

fn css_in(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::String(path) if path.ends_with(".css") && !path.contains('*') => {
            if !found.contains(path) {
                found.push(path.clone());
            }
        }
        Value::Object(fields) => fields.values().for_each(|one| css_in(one, found)),
        Value::Array(list) => list.iter().for_each(|one| css_in(one, found)),
        _ => {}
    }
}

/// The manifest in `text`, or `None` where it is not JSON of the shape.
pub fn read_package_manifest(text_of: &str) -> Option<PackageManifest> {
    let raw = crate::decode::<Raw>("a package manifest", text_of.as_bytes()).ok()?;
    let mut css_exports = Vec::new();
    if let Some(exports) = &raw.exports {
        css_in(exports, &mut css_exports);
    }
    let mut depends_on: Vec<String> = raw
        .dependencies
        .into_keys()
        .chain(raw.dev_dependencies.into_keys())
        .chain(raw.peer_dependencies.into_keys())
        .collect();
    depends_on.sort();
    depends_on.dedup();
    Some(PackageManifest { name: text(raw.name), style: text(raw.style), main: text(raw.main), css_exports, depends_on })
}

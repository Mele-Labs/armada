//! TypeScript for the closure of the types a client reads, from the definitions [`super::parse`] found.
//!
//! **What a type's TypeScript says is what serde puts on the wire**, so a field is optional when
//! `skip_serializing_if` can leave it out, and `default` alone changes nothing: it is how a
//! deserializer treats an absent key, and Fleet still sends the key.
//! The closed sets `core-model` owns are `wire_enum!` types, spelled as strings on the wire, and so
//! `string` here; a union spelled here would be a second copy of a roster that has a registry.

use std::collections::{BTreeMap, BTreeSet};

use super::parse::{Attrs, Body, Field, Item, Shape, Ty, Variant};
use super::rename;

type R<T> = Result<T, String>;

pub struct Registry {
    pub items: Vec<Item>,
}

/// The definitions the generator has no use for a definition of: std and serde_json names it knows.
const BUILTINS: &[&str] = &[
    "String", "str", "char", "PathBuf", "Path", "bool", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16",
    "i32", "i64", "i128", "isize", "f32", "f64", "Option", "Vec", "VecDeque", "BTreeSet", "HashSet", "BTreeMap",
    "HashMap", "Box", "Rc", "Arc", "Cow", "Value", "Map",
];

/// The types whose serde impl is written by hand, and what that impl puts on the wire.
/// A type with no `Serialize` derive that is not here fails the run, so a new one is read before it is trusted.
const WRITTEN_BY_HAND: &[(&str, &str)] = &[
    // `impl Serialize for PageMessages` in `artifact.rs`: `serialize_u8(1)`.
    ("PageMessages", "1"),
];

impl Registry {
    /// The definition `name` means from inside `file`: one in that file first, then the only one there is.
    fn find(&self, name: &str, file: &str) -> R<Option<usize>> {
        let hits: Vec<usize> = (0..self.items.len()).filter(|&i| self.items[i].name == name).collect();
        match hits.len() {
            0 => Ok(None),
            1 => Ok(Some(hits[0])),
            _ => hits
                .iter()
                .copied()
                .find(|&i| self.items[i].file == file)
                .map(Some)
                .ok_or_else(|| format!("`{name}` is defined in more than one file and `{file}` is not one of them")),
        }
    }

    fn refs(&self, item: &Item, out: &mut Vec<Ty>) {
        let fields = |shape: &Shape, out: &mut Vec<Ty>| match shape {
            Shape::Named(fs) | Shape::Tuple(fs) => {
                out.extend(fs.iter().filter(|f| !skipped(f)).map(|f| f.ty.clone()))
            }
            Shape::Unit => {}
        };
        match &item.body {
            Body::Alias(t) => out.push(t.clone()),
            Body::Struct(s) => fields(s, out),
            Body::Enum(vs) => vs.iter().for_each(|v| fields(&v.shape, out)),
        }
        if let Some(into) = item.attrs.value("into") {
            out.push(Ty::Path { name: into.rsplit("::").next().unwrap_or(into).to_string(), args: Vec::new(), json: false });
        }
    }

    /// Every definition reachable from `roots`, in the order the files declare them.
    pub fn closure(&self, roots: &[&str]) -> R<Vec<usize>> {
        let mut seen = BTreeSet::new();
        let mut work: Vec<(usize, String)> = Vec::new();
        for r in roots {
            match self.find(r, "")? {
                Some(i) => work.push((i, String::new())),
                None => return Err(format!("root `{r}` is not defined in crates/ipc/src")),
            }
        }
        while let Some((i, via)) = work.pop() {
            if !seen.insert(i) {
                continue;
            }
            let item = &self.items[i];
            let mut tys = Vec::new();
            self.refs(item, &mut tys);
            let mut names = Vec::new();
            for t in &tys {
                names_in(t, &mut names);
            }
            for (name, json) in names {
                if json || item.generics.contains(&name) {
                    continue;
                }
                match self.find(&name, &item.file)? {
                    Some(j) => work.push((j, item.name.clone())),
                    None if BUILTINS.contains(&name.as_str()) => {}
                    None => return Err(format!("`{}` names `{name}` (reached from `{via}`), which crates/ipc/src does not define", item.name)),
                }
            }
        }
        let mut out: Vec<usize> = seen.into_iter().collect();
        out.sort();
        let mut names = BTreeMap::new();
        for &i in &out {
            if let Some(prev) = names.insert(self.items[i].name.clone(), self.items[i].file.clone()) {
                return Err(format!("`{}` is defined in `{prev}` and in `{}`, and both are reached", self.items[i].name, self.items[i].file));
            }
        }
        Ok(out)
    }
}

fn names_in(t: &Ty, out: &mut Vec<(String, bool)>) {
    match t {
        Ty::Path { name, args, json } => {
            out.push((name.clone(), *json));
            args.iter().for_each(|a| names_in(a, out));
        }
        Ty::Tuple(ts) => ts.iter().for_each(|a| names_in(a, out)),
        Ty::Seq(t) => names_in(t, out),
    }
}

fn skipped(f: &Field) -> bool {
    f.attrs.has("skip") || f.attrs.has("skip_serializing")
}

/// One output file per source file: its path under the wire directory, and what it holds.
pub fn render(reg: &Registry, closure: &[usize]) -> R<Vec<(String, String)>> {
    let mut by_file: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for &i in closure {
        by_file.entry(reg.items[i].file.clone()).or_default().push(i);
    }
    let mut out = Vec::new();
    for (file, members) in &by_file {
        let mut e = Emitter { reg, file, imports: BTreeMap::new() };
        let mut body = String::new();
        for &i in members {
            body.push('\n');
            body.push_str(&e.item(&reg.items[i]).map_err(|why| format!("{}: {why}", reg.items[i].name))?);
            body.push('\n');
        }
        let mut text = format!(
            "// Generated by `cargo xtask verify-wire --write` from `crates/ipc/src/{file}.rs`. Do not edit.\n"
        );
        if !e.imports.is_empty() {
            text.push('\n');
            for (from, names) in &e.imports {
                let names: Vec<&str> = names.iter().map(String::as_str).collect();
                text.push_str(&format!("import type {{ {} }} from \"./{}\";\n", names.join(", "), module(from)));
            }
        }
        text.push_str(&body);
        out.push((format!("{}.ts", module(file)), text));
    }
    Ok(out)
}

/// The module a source file's types are written to.
pub fn module(file: &str) -> String {
    file.replace(['_', '/'], "-")
}

struct Emitter<'a> {
    reg: &'a Registry,
    file: &'a str,
    imports: BTreeMap<String, BTreeSet<String>>,
}

impl Emitter<'_> {
    fn item(&mut self, item: &Item) -> R<String> {
        let a = &item.attrs;
        if matches!(item.body, Body::Struct(_) | Body::Enum(_)) && !a.derives.iter().any(|d| d == "Serialize") {
            let Some((_, written)) = WRITTEN_BY_HAND.iter().find(|(n, _)| *n == item.name) else {
                return Err("has no `Serialize` derive, so its wire shape is written by hand".to_string());
            };
            return Ok(format!("{}export type {} = {written};", doc(&a.docs, ""), item.name));
        }
        let generics = if item.generics.is_empty() { String::new() } else { format!("<{}>", item.generics.join(", ")) };
        let value = if let Some(into) = a.value("into") {
            let name = into.rsplit("::").next().unwrap_or(into);
            self.ty(&Ty::Path { name: name.to_string(), args: Vec::new(), json: false }, item)?
        } else {
            match &item.body {
                Body::Alias(t) => self.ty(t, item)?,
                Body::Struct(shape) => self.shape(shape, a.value("rename_all"), a.has("transparent"), item)?,
                Body::Enum(vs) => self.enumeration(vs, a, item)?,
            }
        };
        let head = format!("export type {}{generics} =", item.name);
        let doc = doc(&a.docs, "");
        let text = if value.starts_with('\n') { format!("{head}{value};") } else { format!("{head} {value};") };
        Ok(format!("{doc}{text}"))
    }

    /// A struct's, or a variant's, fields as one TypeScript type.
    fn shape(&mut self, shape: &Shape, rule: Option<&str>, transparent: bool, item: &Item) -> R<String> {
        match shape {
            Shape::Unit => Ok("null".to_string()),
            Shape::Tuple(fs) => {
                let fs: Vec<&Field> = fs.iter().filter(|f| !skipped(f)).collect();
                if fs.len() == 1 {
                    return self.ty(&fs[0].ty, item);
                }
                if transparent {
                    return Err("`transparent` over more than one field".to_string());
                }
                let parts: R<Vec<String>> = fs.iter().map(|f| self.ty(&f.ty, item)).collect();
                Ok(format!("[{}]", parts?.join(", ")))
            }
            Shape::Named(fs) => {
                let (entries, flat) = self.entries(fs, rule, item)?;
                Ok(compose(entries, flat))
            }
        }
    }

    /// A named list's field entries, and the types it flattens in.
    fn entries(&mut self, fs: &[Field], rule: Option<&str>, item: &Item) -> R<(Vec<String>, Vec<String>)> {
        let mut entries = Vec::new();
        let mut flat = Vec::new();
        for f in fs {
            if skipped(f) {
                continue;
            }
            if f.attrs.has("flatten") {
                flat.push(self.ty(&f.ty, item)?);
                continue;
            }
            entries.push(self.field(f, rule, item)?);
        }
        Ok((entries, flat))
    }

    fn field(&mut self, f: &Field, rule: Option<&str>, item: &Item) -> R<String> {
        let raw = f.name.as_deref().unwrap_or_default();
        let name = match (f.attrs.value("rename"), rule) {
            (Some(n), _) => n.to_string(),
            (None, Some(r)) => rename::field(r, raw)?,
            (None, None) => raw.to_string(),
        };
        let skip = f.attrs.value("skip_serializing_if");
        let option = match &f.ty {
            Ty::Path { name, args, json: false } if name == "Option" && args.len() == 1 => Some(&args[0]),
            _ => None,
        };
        let (optional, ty) = match (option, skip) {
            (Some(inner), Some(s)) if s.ends_with("is_none") => (true, self.ty(inner, item)?),
            (Some(inner), Some(_)) => (true, format!("{} | null", self.ty(inner, item)?)),
            (Some(inner), None) => (false, format!("{} | null", self.ty(inner, item)?)),
            (None, Some(_)) => (true, self.ty(&f.ty, item)?),
            (None, None) => (false, self.ty(&f.ty, item)?),
        };
        let key = if is_identifier(&name) { name } else { format!("\"{name}\"") };
        Ok(format!("{}{key}{}: {ty};", doc(&f.attrs.docs, ""), if optional { "?" } else { "" }))
    }

    fn enumeration(&mut self, vs: &[Variant], a: &Attrs, item: &Item) -> R<String> {
        let rule = a.value("rename_all");
        let fields_rule = a.value("rename_all_fields");
        let tag = a.value("tag");
        let content = a.value("content");
        let untagged = a.has("untagged");
        let mut entries = Vec::new();
        for v in vs {
            if v.attrs.has("skip") || v.attrs.has("skip_serializing") {
                continue;
            }
            let name = match (v.attrs.value("rename"), rule) {
                (Some(n), _) => n.to_string(),
                (None, Some(r)) => rename::variant(r, &v.name)?,
                (None, None) => v.name.clone(),
            };
            let frule = v.attrs.value("rename_all").or(fields_rule);
            let lit = format!("\"{name}\"");
            let ty = if untagged {
                self.shape(&v.shape, frule, false, item)?
            } else if let Some(tag) = tag {
                let tag_entry = format!("{tag}: {lit};");
                match (&v.shape, content) {
                    (Shape::Unit, _) => compose(vec![tag_entry], Vec::new()),
                    (shape, Some(content)) => {
                        let inner = self.shape(shape, frule, false, item)?;
                        compose(vec![tag_entry, format!("{content}: {inner};")], Vec::new())
                    }
                    (Shape::Named(fs), None) => {
                        let (mut own, flat) = self.entries(fs, frule, item)?;
                        own.insert(0, tag_entry);
                        compose(own, flat)
                    }
                    (Shape::Tuple(fs), None) if fs.len() == 1 => {
                        let inner = self.ty(&fs[0].ty, item)?;
                        compose(vec![tag_entry], vec![inner])
                    }
                    (Shape::Tuple(_), None) => return Err(format!("`{}` is a tuple variant of an internally tagged enum", v.name)),
                }
            } else {
                match &v.shape {
                    Shape::Unit => lit,
                    shape => {
                        let inner = self.shape(shape, frule, false, item)?;
                        compose(vec![format!("{}: {inner};", if is_identifier(&name) { name.clone() } else { lit })], Vec::new())
                    }
                }
            };
            entries.push(format!("{}| {}", doc(&v.attrs.docs, ""), indent_tail(&ty, 2)));
        }
        if entries.is_empty() {
            return Ok("never".to_string());
        }
        let body: Vec<String> = entries.iter().map(|e| indent(e, 2)).collect();
        Ok(format!("\n{}", body.join("\n")))
    }

    fn ty(&mut self, ty: &Ty, item: &Item) -> R<String> {
        match ty {
            Ty::Tuple(ts) if ts.is_empty() => Ok("null".to_string()),
            Ty::Tuple(ts) => {
                let parts: R<Vec<String>> = ts.iter().map(|t| self.ty(t, item)).collect();
                Ok(format!("[{}]", parts?.join(", ")))
            }
            Ty::Seq(t) => Ok(array(&self.ty(t, item)?)),
            Ty::Path { name, args, json } => {
                if item.generics.contains(name) {
                    return Ok(name.clone());
                }
                if *json {
                    return match name.as_str() {
                        "Value" => Ok("unknown".to_string()),
                        "Map" => Ok("Record<string, unknown>".to_string()),
                        other => Err(format!("serde_json::{other} has no TypeScript here")),
                    };
                }
                if let Some(i) = self.reg.find(name, &item.file)? {
                    let target = &self.reg.items[i];
                    if target.file != self.file {
                        self.imports.entry(target.file.clone()).or_default().insert(target.name.clone());
                    }
                    if args.is_empty() {
                        return Ok(name.clone());
                    }
                    let parts: R<Vec<String>> = args.iter().map(|t| self.ty(t, item)).collect();
                    return Ok(format!("{name}<{}>", parts?.join(", ")));
                }
                let arg = |me: &mut Self, n: usize| -> R<String> {
                    args.get(n).ok_or_else(|| format!("`{name}` without its argument")).and_then(|t| me.ty(t, item))
                };
                Ok(match name.as_str() {
                    "String" | "str" | "char" | "PathBuf" | "Path" => "string".to_string(),
                    "bool" => "boolean".to_string(),
                    "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
                    | "f32" | "f64" => "number".to_string(),
                    "Option" => format!("{} | null", arg(self, 0)?),
                    "Vec" | "VecDeque" | "BTreeSet" | "HashSet" => array(&arg(self, 0)?),
                    "BTreeMap" | "HashMap" => format!("Record<string, {}>", arg(self, 1)?),
                    "Box" | "Rc" | "Arc" | "Cow" => arg(self, 0)?,
                    "Value" => "unknown".to_string(),
                    other => return Err(format!("`{other}` is not defined in crates/ipc/src")),
                })
            }
        }
    }
}

fn array(inner: &str) -> String {
    if inner.contains(" | ") || inner.contains(" & ") {
        format!("({inner})[]")
    } else {
        format!("{inner}[]")
    }
}

/// Own fields as one object, then what is flattened in.
fn compose(entries: Vec<String>, flat: Vec<String>) -> String {
    let mut parts = Vec::new();
    if !entries.is_empty() {
        let body: Vec<String> = entries.iter().map(|e| indent(e, 2)).collect();
        parts.push(format!("{{\n{}\n}}", body.join("\n")));
    }
    parts.extend(flat);
    if parts.is_empty() {
        "Record<string, never>".to_string()
    } else {
        parts.join(" & ")
    }
}

fn indent(text: &str, by: usize) -> String {
    let pad = " ".repeat(by);
    text.lines().map(|l| if l.is_empty() { String::new() } else { format!("{pad}{l}") }).collect::<Vec<_>>().join("\n")
}

/// Every line after the first, indented.
fn indent_tail(text: &str, by: usize) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default().to_string();
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        first
    } else {
        format!("{first}\n{}", indent(&rest.join("\n"), by))
    }
}

fn is_identifier(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// A TSDoc comment from Rust doc lines, ending in a newline, or nothing.
fn doc(lines: &[String], pad: &str) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = lines.iter().map(|l| l.replace("*/", "*\\/")).collect();
    if lines.len() == 1 {
        return format!("{pad}/** {} */\n", lines[0].trim_end());
    }
    let mut out = format!("{pad}/**\n");
    for l in &lines {
        if l.trim().is_empty() {
            out.push_str(&format!("{pad} *\n"));
        } else {
            out.push_str(&format!("{pad} * {}\n", l.trim_end()));
        }
    }
    out.push_str(&format!("{pad} */\n"));
    out
}

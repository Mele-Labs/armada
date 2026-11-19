//! The type definitions of one Rust file, with the serde attributes that decide their wire shape.
//!
//! **Only what a derive reads is kept.** An item this does not understand is skipped, and a type
//! that something reachable names but nothing here defined is the generator's failure, not a gap
//! it papers over.

use super::lex::Tok;

/// A field or variant's attributes. `serde` holds every `key` and `key = "value"` of every
/// `#[serde(...)]`, in order; `rename(serialize = "x")` is kept as `rename = "x"`.
#[derive(Clone, Debug, Default)]
pub struct Attrs {
    pub docs: Vec<String>,
    pub serde: Vec<(String, Option<String>)>,
    pub cfg_test: bool,
    /// The traits named in `#[derive(...)]`.
    pub derives: Vec<String>,
}

impl Attrs {
    pub fn has(&self, key: &str) -> bool {
        self.serde.iter().any(|(k, _)| k == key)
    }
    pub fn value(&self, key: &str) -> Option<&str> {
        self.serde.iter().find(|(k, _)| k == key).and_then(|(_, v)| v.as_deref())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    /// A named type with its generic arguments, by the last segment of its path.
    Path { name: String, args: Vec<Ty>, json: bool },
    Tuple(Vec<Ty>),
    Seq(Box<Ty>),
}

#[derive(Clone, Debug)]
pub struct Field {
    /// `None` in a tuple.
    pub name: Option<String>,
    pub ty: Ty,
    pub attrs: Attrs,
}

#[derive(Clone, Debug)]
pub enum Shape {
    Named(Vec<Field>),
    Tuple(Vec<Field>),
    Unit,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: String,
    pub attrs: Attrs,
    pub shape: Shape,
}

#[derive(Clone, Debug)]
pub enum Body {
    Struct(Shape),
    Enum(Vec<Variant>),
    Alias(Ty),
}

#[derive(Clone, Debug)]
pub struct Item {
    pub name: String,
    pub generics: Vec<String>,
    pub attrs: Attrs,
    pub body: Body,
    /// The file it was read from, relative to `crates/ipc/src`, without `.rs`.
    pub file: String,
}

pub struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    file: String,
}

type R<T> = Result<T, String>;

pub fn items(toks: Vec<Tok>, file: &str) -> R<Vec<Item>> {
    let mut p = Parser { toks, pos: 0, file: file.to_string() };
    p.items().map_err(|why| format!("{file}: {why}"))
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn peek_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }
    fn is_punct(&self, c: char) -> bool {
        self.peek() == Some(&Tok::Punct(c))
    }
    fn is_ident(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(i)) if i == s)
    }
    fn eat_punct(&mut self, c: char) -> bool {
        let hit = self.is_punct(c);
        if hit {
            self.pos += 1;
        }
        hit
    }
    fn expect_punct(&mut self, c: char) -> R<()> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            Err(format!("expected `{c}`, found {:?}, after {}", self.peek(), self.recent()))
        }
    }
    fn ident(&mut self) -> R<String> {
        match self.peek() {
            Some(Tok::Ident(i)) => {
                let i = i.clone();
                self.pos += 1;
                Ok(i)
            }
            other => Err(format!("expected a name, found {other:?}, after {}", self.recent())),
        }
    }

    /// The last few tokens, for an error to say where it was.
    fn recent(&self) -> String {
        let from = self.pos.saturating_sub(8);
        format!("{:?}", &self.toks[from..self.pos.min(self.toks.len())])
    }

    /// Skips one bracketed group, the cursor on its opener.
    fn skip_group(&mut self) -> R<()> {
        let mut depth = 0i32;
        while let Some(t) = self.peek() {
            if let Tok::Punct(c) = t {
                match c {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => depth -= 1,
                    _ => {}
                }
            }
            self.pos += 1;
            if depth == 0 {
                return Ok(());
            }
        }
        Err("a group never closes".to_string())
    }

    fn items(&mut self) -> R<Vec<Item>> {
        let mut out = Vec::new();
        while self.peek().is_some() {
            let attrs = self.attrs()?;
            if self.peek().is_none() {
                break;
            }
            self.vis()?;
            let Some(Tok::Ident(kw)) = self.peek().cloned() else {
                self.pos += 1;
                continue;
            };
            match kw.as_str() {
                "struct" | "enum" | "type" => {
                    self.pos += 1;
                    let item = self.definition(&kw, attrs)?;
                    if !item.attrs.cfg_test {
                        out.push(item);
                    }
                }
                name if WIRE_MACROS.contains(&name) && self.peek_at(1) == Some(&Tok::Punct('!')) => {
                    self.pos += 2;
                    out.push(self.macro_string()?);
                }
                _ => self.skip_item(&kw)?,
            }
        }
        Ok(out)
    }

    /// `wire_id! { /// docs  Name, core_model::Name }`: a type the wire spells as a string.
    fn macro_string(&mut self) -> R<Item> {
        let start = self.pos;
        self.skip_group()?;
        let end = self.pos;
        self.eat_punct(';');
        let inner = Parser { toks: self.toks[start + 1..end - 1].to_vec(), pos: 0, file: self.file.clone() };
        let mut inner = inner;
        let attrs = inner.attrs()?;
        let name = inner.ident()?;
        Ok(Item {
            name,
            generics: Vec::new(),
            attrs,
            body: Body::Alias(Ty::Path { name: "String".into(), args: Vec::new(), json: false }),
            file: self.file.clone(),
        })
    }

    fn definition(&mut self, kw: &str, attrs: Attrs) -> R<Item> {
        let name = self.ident()?;
        let generics = self.generics()?;
        let body = match kw {
            "type" => {
                self.expect_punct('=')?;
                let ty = self.ty()?;
                self.expect_punct(';')?;
                Body::Alias(ty)
            }
            "struct" => {
                let shape = if self.is_punct('{') {
                    Shape::Named(self.fields(true)?)
                } else if self.is_punct('(') {
                    let f = self.fields(false)?;
                    self.eat_punct(';');
                    Shape::Tuple(f)
                } else {
                    self.expect_punct(';')?;
                    Shape::Unit
                };
                Body::Struct(shape)
            }
            _ => {
                self.expect_punct('{')?;
                let mut variants = Vec::new();
                while !self.is_punct('}') {
                    let attrs = self.attrs()?;
                    let name = self.ident()?;
                    let shape = if self.is_punct('{') {
                        Shape::Named(self.fields(true)?)
                    } else if self.is_punct('(') {
                        Shape::Tuple(self.fields(false)?)
                    } else {
                        Shape::Unit
                    };
                    if self.eat_punct('=') {
                        while !self.is_punct(',') && !self.is_punct('}') {
                            self.pos += 1;
                        }
                    }
                    self.eat_punct(',');
                    variants.push(Variant { name, attrs, shape });
                }
                self.expect_punct('}')?;
                Body::Enum(variants)
            }
        };
        Ok(Item { name, generics, attrs, body, file: self.file.clone() })
    }

    /// `<T, U: Bound, 'a>` as the type parameters' names.
    fn generics(&mut self) -> R<Vec<String>> {
        let mut names = Vec::new();
        if !self.eat_punct('<') {
            return Ok(names);
        }
        let mut depth = 1;
        let mut at_start = true;
        while depth > 0 {
            match self.peek().cloned() {
                Some(Tok::Punct('<')) => depth += 1,
                Some(Tok::Punct('>')) => depth -= 1,
                Some(Tok::Punct(',')) if depth == 1 => at_start = true,
                Some(Tok::Ident(i)) if depth == 1 && at_start => {
                    at_start = i == "const";
                    if !i.starts_with('\'') && i != "const" {
                        names.push(i);
                    }
                }
                Some(_) => at_start = false,
                None => return Err("generics never close".to_string()),
            }
            self.pos += 1;
        }
        Ok(names)
    }

    /// The fields of a braced or parenthesised list, the cursor on the opener.
    fn fields(&mut self, named: bool) -> R<Vec<Field>> {
        self.pos += 1;
        let close = if named { '}' } else { ')' };
        let mut out = Vec::new();
        while !self.is_punct(close) {
            let attrs = self.attrs()?;
            self.vis()?;
            let name = if named {
                let n = self.ident()?;
                self.expect_punct(':')?;
                Some(n)
            } else {
                None
            };
            let ty = self.ty()?;
            self.eat_punct(',');
            out.push(Field { name, ty, attrs });
        }
        self.pos += 1;
        Ok(out)
    }

    pub fn ty(&mut self) -> R<Ty> {
        if self.eat_punct('&') {
            if matches!(self.peek(), Some(Tok::Ident(i)) if i.starts_with('\'')) {
                self.pos += 1;
            }
            if self.is_ident("mut") {
                self.pos += 1;
            }
            return self.ty();
        }
        if self.eat_punct('(') {
            let mut parts = Vec::new();
            while !self.is_punct(')') {
                parts.push(self.ty()?);
                self.eat_punct(',');
            }
            self.pos += 1;
            return Ok(Ty::Tuple(parts));
        }
        if self.eat_punct('[') {
            let inner = self.ty()?;
            if self.eat_punct(';') {
                while !self.is_punct(']') {
                    self.pos += 1;
                }
            }
            self.expect_punct(']')?;
            return Ok(Ty::Seq(Box::new(inner)));
        }
        let mut name = self.ident()?;
        if matches!(name.as_str(), "dyn" | "impl") {
            return Err(format!("`{name}` is not a wire type"));
        }
        let mut json = name == "serde_json";
        let mut args = Vec::new();
        loop {
            if self.eat_punct(':') {
                self.expect_punct(':')?;
                name = self.ident()?;
                json |= name == "serde_json";
                continue;
            }
            if self.is_punct('<') {
                self.pos += 1;
                args.clear();
                while !self.is_punct('>') {
                    if matches!(self.peek(), Some(Tok::Ident(i)) if i.starts_with('\'')) || self.peek() == Some(&Tok::Lit) {
                        self.pos += 1;
                    } else {
                        args.push(self.ty()?);
                    }
                    self.eat_punct(',');
                }
                self.pos += 1;
                continue;
            }
            break;
        }
        Ok(Ty::Path { name, args, json })
    }

    /// `pub`, `pub(crate)`, `pub(in path)`.
    fn vis(&mut self) -> R<()> {
        if self.is_ident("pub") {
            self.pos += 1;
            if self.is_punct('(') {
                self.skip_group()?;
            }
        }
        Ok(())
    }

    /// Doc comments and `#[...]` attributes, and `#![...]` ones, which are dropped.
    fn attrs(&mut self) -> R<Attrs> {
        let mut attrs = Attrs::default();
        loop {
            match self.peek().cloned() {
                Some(Tok::Doc(line)) => {
                    attrs.docs.push(line);
                    self.pos += 1;
                }
                Some(Tok::Punct('#')) => {
                    self.pos += 1;
                    self.eat_punct('!');
                    let start = self.pos;
                    self.skip_group()?;
                    let body: Vec<Tok> = self.toks[start + 1..self.pos - 1].to_vec();
                    read_attr(&body, &mut attrs)?;
                }
                _ => return Ok(attrs),
            }
        }
    }

    /// Skips an item this reader has no use for: a function, an impl, a `use`, a constant, a module.
    fn skip_item(&mut self, kw: &str) -> R<()> {
        let to_semicolon = matches!(kw, "use" | "static") || (kw == "const" && !self.fn_ahead());
        if kw == "macro_rules" {
            self.pos += 3;
            self.skip_group()?;
            self.eat_punct(';');
            return Ok(());
        }
        while let Some(t) = self.peek() {
            match t {
                Tok::Punct(';') => {
                    self.pos += 1;
                    return Ok(());
                }
                Tok::Punct('{') if !to_semicolon => return self.skip_group(),
                Tok::Punct('(' | '[' | '{') => self.skip_group()?,
                _ => self.pos += 1,
            }
        }
        Ok(())
    }

    fn fn_ahead(&self) -> bool {
        matches!(self.peek_at(1), Some(Tok::Ident(i)) if i == "fn" || i == "unsafe" || i == "async")
    }
}

/// The macros that define a type the wire spells as a string: `crates/ipc/src/ids.rs` and `enums.rs`.
const WIRE_MACROS: &[&str] = &["wire_id", "wire_enum"];

fn read_attr(body: &[Tok], attrs: &mut Attrs) -> R<()> {
    let Some(Tok::Ident(head)) = body.first() else {
        return Ok(());
    };
    match head.as_str() {
        "cfg" => {
            if body.iter().any(|t| matches!(t, Tok::Ident(i) if i == "test")) {
                attrs.cfg_test = true;
            }
        }
        "derive" => {
            for t in &body[1..] {
                if let Tok::Ident(i) = t {
                    attrs.derives.push(i.clone());
                }
            }
        }
        "serde" => {
            let mut i = 2;
            while i < body.len() {
                let Tok::Ident(key) = &body[i] else {
                    i += 1;
                    continue;
                };
                let key = key.clone();
                i += 1;
                match body.get(i) {
                    Some(Tok::Punct('=')) => {
                        let Some(Tok::Str(v)) = body.get(i + 1) else {
                            return Err(format!("serde `{key}` has a value that is not a string"));
                        };
                        attrs.serde.push((key, Some(v.clone())));
                        i += 2;
                    }
                    Some(Tok::Punct('(')) => {
                        let mut depth = 0;
                        let mut serialize = None;
                        while i < body.len() {
                            match &body[i] {
                                Tok::Punct('(') => depth += 1,
                                Tok::Punct(')') => depth -= 1,
                                Tok::Ident(k) if k == "serialize" => {
                                    if let Some(Tok::Str(v)) = body.get(i + 2) {
                                        serialize = Some(v.clone());
                                    }
                                }
                                _ => {}
                            }
                            i += 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        attrs.serde.push((key, serialize));
                    }
                    _ => attrs.serde.push((key, None)),
                }
            }
        }
        _ => {}
    }
    Ok(())
}

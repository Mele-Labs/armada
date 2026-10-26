//! A body's whole shape, read off the type that decodes it, so a refusal names every field at once.
//! serde's own error names the first field wrong, and an agent learning a body from refusals paid
//! one call a field.
//!
//! **Read, never written down.** The type is driven through a deserializer of this module's that
//! answers nothing and notes what was asked for: a struct names its fields, an enum its variants, a
//! sequence or an option its element. One pass reads one place in the type, so the shape costs a
//! pass a field and is only read on a refusal. A field is optional where the type builds without
//! it: each required one is found as the `missing field` the type names when given the others.
//!
//! What the type does not say is read as `any`: an untagged enum, a field read by hand, and the
//! variants' own fields of an internally tagged enum, whose tag and its values are still named.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fmt;

use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, MapAccess,
    SeqAccess, VariantAccess, Visitor,
};

use filler::Anything;

/// The values a struct is given when a pass asks whether it builds without a field.
mod filler;

/// Deeper than this, a place reads as `object`: a type that holds itself would never end.
const DEEPEST: usize = 6;

/// `T`'s shape as one line, such as `{text: string, enter?: boolean}`.
pub fn of<T: DeserializeOwned>() -> String {
    render(&node::<T>(&mut Vec::new(), &mut Vec::new()), None, &mut Vec::new())
}

/// One step down from a place in the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Field(&'static str),
    Item,
    Value,
    Variant(&'static str),
    Tag(&'static str),
}

/// What the type asked for at the place a pass read.
#[derive(Clone, Copy, Debug)]
enum Seen {
    Leaf(&'static str),
    List,
    Map,
    Struct(Kind),
    Enum(&'static [&'static str]),
    Tagged(&'static str),
    /// The values of an internally tagged enum's tag.
    Tags(&'static [&'static str]),
    /// A struct given some of its fields built, or named the one missing first.
    Built,
    Missing(&'static str),
}

#[derive(Debug)]
enum Node {
    Leaf(&'static str),
    List(Box<Node>),
    Map(Box<Node>),
    Struct(Kind, Vec<(&'static str, bool, Node)>),
    /// A struct inside itself, such as a locator's `within`.
    Again,
    /// Each variant, with what it carries where it carries something.
    Choice(Vec<(&'static str, Option<Node>)>),
    Tagged(&'static str, &'static [&'static str]),
}

/// A struct type: its name and its fields, which together tell two types apart.
#[derive(Clone, Copy, Debug)]
struct Kind(&'static str, &'static [&'static str]);

impl PartialEq for Kind {
    fn eq(&self, other: &Kind) -> bool {
        self.0 == other.0 && std::ptr::eq(self.1, other.1)
    }
}

/// Why a pass stopped. Only the three serde raises by name are told apart.
#[derive(Debug)]
enum Halt {
    Stop,
    Missing(&'static str),
    Variants(&'static [&'static str]),
}

impl fmt::Display for Halt {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "{self:?}")
    }
}

impl std::error::Error for Halt {}

impl de::Error for Halt {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Halt::Stop
    }
    fn missing_field(field: &'static str) -> Self {
        Halt::Missing(field)
    }
    fn unknown_variant(_: &str, expected: &'static [&'static str]) -> Self {
        Halt::Variants(expected)
    }
}

#[derive(Default)]
struct Notes {
    seen: RefCell<Option<Seen>>,
    /// An option stood at the place, so the field is optional whatever the passes say.
    optional: Cell<bool>,
}

/// One pass, read at [`Step`]s down from `T`.
fn look<T: DeserializeOwned>(path: &[Step], given: Option<&[&'static str]>) -> (Option<Seen>, bool) {
    let notes = Notes::default();
    let _ = T::deserialize(Probe { path, given, notes: &notes });
    (notes.seen.into_inner(), notes.optional.get())
}

fn node<T: DeserializeOwned>(path: &mut Vec<Step>, within: &mut Vec<Kind>) -> Node {
    node_and_optional::<T>(path, within).0
}

/// The place at `path`, and whether an option stood there. `within` is the structs above it.
fn node_and_optional<T: DeserializeOwned>(path: &mut Vec<Step>, within: &mut Vec<Kind>) -> (Node, bool) {
    if path.len() > DEEPEST {
        return (Node::Leaf("object"), false);
    }
    let (seen, optional) = look::<T>(path, None);
    if let Some(Seen::Struct(kind)) = seen {
        if within.contains(&kind) {
            return (Node::Again, optional);
        }
        within.push(kind);
    }
    let mut below = |step: Step| {
        path.push(step);
        let below = node_and_optional::<T>(path, within);
        path.pop();
        below
    };
    let node = match seen {
        Some(Seen::List) => Node::List(Box::new(below(Step::Item).0)),
        Some(Seen::Map) => Node::Map(Box::new(below(Step::Value).0)),
        Some(Seen::Enum(variants)) => Node::Choice(
            variants
                .iter()
                .map(|variant| match below(Step::Variant(variant)).0 {
                    Node::Leaf("null") => (*variant, None),
                    carried => (*variant, Some(carried)),
                })
                .collect(),
        ),
        Some(Seen::Struct(kind)) => {
            let fields = kind.1;
            let read: Vec<(Node, bool)> = fields.iter().map(|field| below(Step::Field(field))).collect();
            within.pop();
            let optional: Vec<bool> = read.iter().map(|(_, optional)| *optional).collect();
            let required = required::<T>(path, fields, &optional);
            Node::Struct(
                kind,
                fields.iter().zip(read).zip(required).map(|((field, (node, _)), required)| (*field, required, node)).collect(),
            )
        }
        Some(Seen::Tagged(tag)) => {
            path.push(Step::Tag(tag));
            let (tags, _) = look::<T>(path, None);
            path.pop();
            match tags {
                Some(Seen::Tags(tags)) => Node::Tagged(tag, tags),
                _ => Node::Tagged(tag, &[]),
            }
        }
        Some(Seen::Leaf(leaf)) => Node::Leaf(leaf),
        _ => Node::Leaf("any"),
    };
    (node, optional)
}

/// Which of `fields` the struct at `path` cannot build without. The type names the first missing
/// one in the order the fields are declared, so each is found by giving every one found before it.
/// Where a pass cannot tell, past the last one found a field is required unless it is an option.
fn required<T: DeserializeOwned>(path: &[Step], fields: &[&'static str], optional: &[bool]) -> Vec<bool> {
    let mut required = vec![false; fields.len()];
    let mut given = Vec::new();
    let mut from = 0;
    for _ in 0..=fields.len() {
        match look::<T>(path, Some(&given)).0 {
            Some(Seen::Built) => return required,
            Some(Seen::Missing(field)) if !given.contains(&field) => {
                let Some(at) = fields.iter().position(|one| *one == field) else { break };
                required[at] = true;
                from = at + 1;
                given.push(field);
            }
            _ => break,
        }
    }
    for at in from..fields.len() {
        required[at] = !optional[at];
    }
    required
}

/// `node` as one line. `field` is the field it is the value of; a struct type met again under
/// another field is written `same as` the first, which `shown` holds.
fn render(node: &Node, field: Option<&'static str>, shown: &mut Vec<(Kind, &'static str)>) -> String {
    match node {
        Node::Leaf(leaf) => (*leaf).to_string(),
        Node::Again => String::from("{...}"),
        Node::List(item) => format!("[{}]", render(item, field, shown)),
        Node::Map(value) => match value.as_ref() {
            Node::Leaf("any") => String::from("object"),
            value => format!("{{string: {}}}", render(value, field, shown)),
        },
        Node::Struct(kind, fields) => {
            if let Some((_, first)) = shown.iter().find(|(one, _)| one == kind) {
                return format!("same as {first}");
            }
            if let Some(field) = field.filter(|_| fields.len() > 2) {
                shown.push((*kind, field));
            }
            let fields: Vec<String> = fields
                .iter()
                .map(|(name, required, node)| {
                    format!("{name}{}: {}", if *required { "" } else { "?" }, render(node, Some(name), shown))
                })
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
        Node::Choice(variants) => {
            let variants: Vec<String> = variants
                .iter()
                .map(|(variant, carried)| match carried {
                    None => (*variant).to_string(),
                    Some(carried) => format!("{{{variant}: {}}}", render(carried, Some(variant), shown)),
                })
                .collect();
            variants.join("|")
        }
        Node::Tagged(tag, tags) if tags.is_empty() => format!("{{{tag}: string, ...}}"),
        Node::Tagged(tag, tags) => format!("{{{tag}: {}, ...}}", tags.join("|")),
    }
}

/// The deserializer a pass drives the type through. It follows `path` down, and where the path
/// ends it notes what was asked for and stops; `given` turns the note on a struct into whether it
/// builds from those fields alone.
struct Probe<'p> {
    path: &'p [Step],
    given: Option<&'p [&'static str]>,
    notes: &'p Notes,
}

impl<'p> Probe<'p> {
    fn rest(&self) -> Probe<'p> {
        Probe { path: &self.path[1..], given: self.given, notes: self.notes }
    }

    fn saw<T>(self, seen: Seen) -> Result<T, Halt> {
        if self.path.is_empty() {
            *self.notes.seen.borrow_mut() = Some(seen);
        }
        Err(Halt::Stop)
    }
}

macro_rules! leaves {
    ($($method:ident => $leaf:literal),* $(,)?) => {
        $(fn $method<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Halt> {
            self.saw(Seen::Leaf($leaf))
        })*
    };
}

impl<'de, 'p> Deserializer<'de> for Probe<'p> {
    type Error = Halt;

    leaves! {
        deserialize_bool => "boolean",
        deserialize_i8 => "number", deserialize_i16 => "number", deserialize_i32 => "number",
        deserialize_i64 => "number", deserialize_i128 => "number", deserialize_u8 => "number",
        deserialize_u16 => "number", deserialize_u32 => "number", deserialize_u64 => "number",
        deserialize_u128 => "number", deserialize_f32 => "number", deserialize_f64 => "number",
        deserialize_char => "string", deserialize_str => "string", deserialize_string => "string",
        deserialize_bytes => "string", deserialize_byte_buf => "string", deserialize_unit => "null",
        deserialize_identifier => "string", deserialize_ignored_any => "any",
    }

    /// An internally tagged enum asks for anything, and given nothing names its tag as missing.
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        match self.path {
            [] => match visitor.visit_map(Entries::new()) {
                Err(Halt::Missing(tag)) => self.saw(Seen::Tagged(tag)),
                _ => self.saw(Seen::Leaf("any")),
            },
            [Step::Tag(tag)] => {
                let mut asked = Entries::new();
                asked.pending.push_back((tag, Given::Word("\u{0}")));
                match visitor.visit_map(asked) {
                    Err(Halt::Variants(tags)) => Probe { path: &[], ..self }.saw(Seen::Tags(tags)),
                    _ => Err(Halt::Stop),
                }
            }
            _ => Err(Halt::Stop),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        if self.path.is_empty() {
            self.notes.optional.set(true);
        }
        visitor.visit_some(self)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Halt> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        match self.path.first() {
            Some(Step::Item) => visitor.visit_seq(Items(Some(self.rest()))),
            _ => self.saw(Seen::List),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _: &'static str, _: usize, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Halt> {
        match self.path.first() {
            Some(Step::Value) => {
                let mut one = Entries::new();
                one.pending.push_back(("key", Given::Probe(self.rest())));
                visitor.visit_map(one)
            }
            _ => self.saw(Seen::Map),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Halt> {
        match (self.path.first(), self.given) {
            (Some(Step::Field(field)), _) => {
                let mut one = Entries::new();
                one.pending.push_back((field, Given::Probe(self.rest())));
                visitor.visit_map(one)
            }
            (None, Some(given)) => {
                let mut those = Entries::new();
                those.pending.extend(given.iter().map(|field| (*field, Given::Anything)));
                let seen = match visitor.visit_map(those) {
                    Ok(_) => Seen::Built,
                    Err(Halt::Missing(field)) => Seen::Missing(field),
                    Err(_) => return Err(Halt::Stop),
                };
                self.saw(seen)
            }
            _ => self.saw(Seen::Struct(Kind(name, fields))),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Halt> {
        match self.path.first() {
            Some(Step::Variant(variant)) => visitor.visit_enum(Picked { variant, then: self.rest() }),
            _ => self.saw(Seen::Enum(variants)),
        }
    }
}

/// What a map entry's value is read from.
enum Given<'p> {
    Probe(Probe<'p>),
    Word(&'static str),
    Anything,
}

struct Entries<'p> {
    pending: VecDeque<(&'static str, Given<'p>)>,
    value: Option<Given<'p>>,
}

impl<'p> Entries<'p> {
    fn new() -> Entries<'p> {
        Entries { pending: VecDeque::new(), value: None }
    }
}

impl<'de, 'p> MapAccess<'de> for Entries<'p> {
    type Error = Halt;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Halt> {
        let Some((key, value)) = self.pending.pop_front() else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(key.into_deserializer()).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Halt> {
        match self.value.take() {
            Some(Given::Probe(probe)) => seed.deserialize(probe),
            Some(Given::Word(word)) => seed.deserialize(word.into_deserializer()),
            Some(Given::Anything) | None => seed.deserialize(Anything),
        }
    }
}

/// A sequence of the one element a pass reads.
struct Items<'p>(Option<Probe<'p>>);

impl<'de, 'p> SeqAccess<'de> for Items<'p> {
    type Error = Halt;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Halt> {
        self.0.take().map(|probe| seed.deserialize(probe)).transpose()
    }
}

/// The variant a pass reads, and the probe for what it carries.
struct Picked<'p> {
    variant: &'static str,
    then: Probe<'p>,
}

impl<'de, 'p> EnumAccess<'de> for Picked<'p> {
    type Error = Halt;
    type Variant = Probe<'p>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Probe<'p>), Halt> {
        let picked = seed.deserialize(self.variant.into_deserializer())?;
        Ok((picked, self.then))
    }
}

impl<'de, 'p> VariantAccess<'de> for Probe<'p> {
    type Error = Halt;

    fn unit_variant(self) -> Result<(), Halt> {
        self.saw(Seen::Leaf("null"))
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Halt> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Halt> {
        self.deserialize_struct("", fields, visitor)
    }
}

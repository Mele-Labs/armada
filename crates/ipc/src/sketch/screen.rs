//! The layout tree of a `screen` node, checked against `readScreen` in
//! `packages/components/src/compositions/SketchScene/screen.ts`. `tests/sketch.rs` reads that file
//! and fails when the types, their fields, the readers or the enums below differ from it.

use serde_json::{Map, Value};

use super::{kind_of, seen, unknown};

/// Most levels of elements one screen nests, the root being the first. A real screen is under half.
pub const MOST_SCREEN_DEPTH: usize = 12;

pub const SCREEN_GAPS: [&str; 4] = ["none", "sm", "md", "lg"];
pub const SCREEN_ALIGNS: [&str; 4] = ["start", "center", "end", "stretch"];
pub const SCREEN_CHROMES: [&str; 3] = ["desktop", "browser", "phone"];
pub const SCREEN_VARIANTS: [&str; 3] = ["primary", "secondary", "ghost"];
pub const SCREEN_FIELDS: [&str; 6] = ["input", "textarea", "select", "checkbox", "toggle", "search"];
pub const SCREEN_TEXTS: [&str; 4] = ["heading", "title", "body", "caption"];
pub const SCREEN_ICONS: &[&str] = &[
    "arrow-left", "arrow-right", "arrow-up-from-line", "bell", "bookmark", "bot", "check", "chevron-down",
    "chevron-left", "chevron-right", "chevron-up", "circle-check", "circle-help", "clock", "copy", "database",
    "download", "ellipsis", "external-link", "eye", "file-text", "files", "filter", "flag", "folder",
    "folder-plus", "git-branch", "globe", "history", "house", "image", "key", "layers", "layout-grid", "link",
    "list-checks", "lock", "mail", "menu", "message-square", "monitor", "package", "panel-left-open", "paperclip",
    "pause", "pencil", "pin", "play", "plus", "refresh-cw", "search", "send", "settings", "sliders-horizontal",
    "smartphone", "terminal", "thumbs-down", "thumbs-up", "ticket", "trash-2", "triangle-alert", "user-check",
    "x",
];
const FROMS: [&str; 2] = ["me", "them"];

/// How one field is read. Each is one reader of `screen.ts`, and the test compares them by name.
#[derive(Clone, Copy)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum Read {
    Text,
    Flag,
    Number,
    Whole(u8, u8),
    Size,
    /// The name of the list in `screen.ts` and the list.
    Choice(&'static str, &'static [&'static str]),
    Icon,
    Act,
    Many(&'static Read),
    Row,
    Message,
    Children,
}

pub struct Spec {
    pub name: &'static str,
    /// Takes `gap`, `pad`, `border` and `children`, as `LAYOUT` does in `screen.ts`.
    pub layout: bool,
    pub fields: &'static [(&'static str, Read)],
    pub required: &'static [&'static str],
}

const GAP: Read = Read::Choice("SCREEN_GAPS", &SCREEN_GAPS);
const ACTS: Read = Read::Many(&Read::Act);
const TEXTS: Read = Read::Many(&Read::Text);

/// What every element takes.
pub const BASE: [(&str, Read); 4] =
    [("id", Read::Text), ("step", Read::Number), ("size", Read::Size), ("align", Read::Choice("SCREEN_ALIGNS", &SCREEN_ALIGNS))];
pub const LAYOUT: [(&str, Read); 4] = [("gap", GAP), ("pad", GAP), ("border", Read::Flag), ("children", Read::Children)];
const ACT_FIELDS: [(&str, Read); 2] = [("text", Read::Text), ("icon", Read::Icon)];
pub const ROW_FIELDS: [(&str, Read); 4] =
    [("text", Read::Text), ("meta", Read::Text), ("icon", Read::Icon), ("selected", Read::Flag)];
pub const MESSAGE_FIELDS: [(&str, Read); 2] =
    [("from", Read::Choice("", &FROMS)), ("text", Read::Text)];

pub const SPECS: [Spec; 19] = [
    Spec { name: "row", layout: true, fields: &[], required: &["children"] },
    Spec { name: "column", layout: true, fields: &[], required: &["children"] },
    Spec { name: "grid", layout: true, fields: &[("cols", Read::Whole(1, 12))], required: &["children", "cols"] },
    Spec {
        name: "window",
        layout: false,
        fields: &[
            ("chrome", Read::Choice("SCREEN_CHROMES", &SCREEN_CHROMES)),
            ("title", Read::Text),
            ("url", Read::Text),
            ("children", Read::Children),
        ],
        required: &["chrome", "children"],
    },
    Spec {
        name: "nav",
        layout: false,
        fields: &[("brand", Read::Text), ("items", ACTS), ("active", Read::Text), ("actions", ACTS), ("vertical", Read::Flag)],
        required: &[],
    },
    Spec {
        name: "panel",
        layout: false,
        fields: &[("title", Read::Text), ("icon", Read::Icon), ("actions", ACTS), ("gap", GAP), ("pad", GAP), ("children", Read::Children)],
        required: &[],
    },
    Spec { name: "tabs", layout: false, fields: &[("items", TEXTS), ("active", Read::Text)], required: &["items"] },
    Spec { name: "list", layout: false, fields: &[("rows", Read::Many(&Read::Row))], required: &["rows"] },
    Spec { name: "table", layout: false, fields: &[("columns", TEXTS), ("rows", Read::Many(&TEXTS))], required: &["columns", "rows"] },
    Spec {
        name: "button",
        layout: false,
        fields: &[("text", Read::Text), ("variant", Read::Choice("SCREEN_VARIANTS", &SCREEN_VARIANTS)), ("icon", Read::Icon)],
        required: &[],
    },
    Spec {
        name: "field",
        layout: false,
        fields: &[
            ("label", Read::Text),
            ("kind", Read::Choice("SCREEN_FIELDS", &SCREEN_FIELDS)),
            ("value", Read::Text),
            ("placeholder", Read::Text),
        ],
        required: &[],
    },
    Spec {
        name: "text",
        layout: false,
        fields: &[("style", Read::Choice("SCREEN_TEXTS", &SCREEN_TEXTS)), ("text", Read::Text), ("lines", Read::Whole(1, 12))],
        required: &[],
    },
    Spec { name: "image", layout: false, fields: &[("label", Read::Text)], required: &[] },
    Spec { name: "avatar", layout: false, fields: &[("text", Read::Text)], required: &[] },
    Spec { name: "badge", layout: false, fields: &[("text", Read::Text)], required: &["text"] },
    Spec { name: "divider", layout: false, fields: &[], required: &[] },
    Spec { name: "spacer", layout: false, fields: &[], required: &[] },
    Spec { name: "chat", layout: false, fields: &[("messages", Read::Many(&Read::Message))], required: &["messages"] },
    Spec { name: "modal", layout: false, fields: &[("title", Read::Text), ("children", Read::Children)], required: &["children"] },
];

/// Everything a spec's element takes besides its `type`.
pub fn fields_of(spec: &Spec) -> Vec<(&'static str, Read)> {
    let layout: &[(&str, Read)] = if spec.layout { &LAYOUT } else { &[] };
    BASE.iter().chain(layout).chain(spec.fields).copied().collect()
}

/// What is wrong with the layout tree at `at`, pushed onto `problems`.
pub(super) fn problems(tree: &Value, at: &str, problems: &mut Vec<String>) {
    element(tree, at, 1, problems);
}

fn element(value: &Value, at: &str, depth: usize, problems: &mut Vec<String>) {
    let Value::Object(fields) = value else {
        problems.push(format!("{at}: must be an object, is {}", kind_of(value)));
        return;
    };
    let named = fields.get("type").and_then(Value::as_str);
    let Some(spec) = SPECS.iter().find(|spec| Some(spec.name) == named) else {
        let types: Vec<&str> = SPECS.iter().map(|spec| spec.name).collect();
        problems.push(format!("{at}.type is {}. Allowed: {}", seen(fields.get("type")), types.join(", ")));
        return;
    };
    if depth > MOST_SCREEN_DEPTH {
        problems.push(format!("{at}: nests {depth} levels deep and a screen holds at most {MOST_SCREEN_DEPTH}"));
        return;
    }
    let mut taken = vec![("type", Read::Text)];
    taken.extend(fields_of(spec));
    record(fields, at, &taken, spec.required, depth, problems);
}

/// An object with these fields and no others, the required ones present and each read.
fn record(
    fields: &Map<String, Value>,
    at: &str,
    taken: &[(&str, Read)],
    required: &[&str],
    depth: usize,
    problems: &mut Vec<String>,
) {
    let names: Vec<&str> = taken.iter().map(|(name, _)| *name).collect();
    unknown(fields.keys(), &names, at, problems);
    for (name, read) in taken {
        match fields.get(*name) {
            None if required.contains(name) => problems.push(format!("{at}: needs `{name}`, {}", wants(*read))),
            None => {}
            Some(value) => field(*read, value, &format!("{at}.{name}"), depth, problems),
        }
    }
}

fn field(read: Read, value: &Value, at: &str, depth: usize, problems: &mut Vec<String>) {
    let fits = match read {
        Read::Text => value.is_string(),
        Read::Flag => value.is_boolean(),
        Read::Number => value.is_number(),
        Read::Whole(least, most) => {
            value.as_f64().is_some_and(|n| n.fract() == 0.0 && n >= f64::from(least) && n <= f64::from(most))
        }
        Read::Size => match value {
            Value::String(said) => said == "auto" || said == "fill",
            other => other.as_f64().is_some_and(|n| n > 0.0),
        },
        Read::Choice(_, list) => value.as_str().is_some_and(|said| list.contains(&said)),
        Read::Icon => value.as_str().is_some_and(|said| SCREEN_ICONS.contains(&said)),
        Read::Act => match value {
            Value::String(_) => true,
            Value::Object(fields) => return record(fields, at, &ACT_FIELDS, &[], depth, problems),
            _ => false,
        },
        Read::Row => match value {
            Value::Object(fields) => return record(fields, at, &ROW_FIELDS, &["text"], depth, problems),
            _ => false,
        },
        Read::Message => match value {
            Value::Object(fields) => return record(fields, at, &MESSAGE_FIELDS, &["from", "text"], depth, problems),
            _ => false,
        },
        Read::Many(one) => match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    field(*one, item, &format!("{at}[{index}]"), depth, problems);
                }
                return;
            }
            _ => false,
        },
        Read::Children => match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    element(item, &format!("{at}[{index}]"), depth + 1, problems);
                }
                return;
            }
            _ => false,
        },
    };
    if !fits {
        let allowed = match read {
            Read::Choice(_, list) => Some(list),
            Read::Icon => Some(SCREEN_ICONS),
            _ => None,
        };
        problems.push(match allowed {
            Some(list) => format!("{at} is {}. Allowed: {}", seen(Some(value)), list.join(", ")),
            None => format!("{at} is {}. It must be {}", seen(Some(value)), wants(read)),
        });
    }
}

/// What a field holds, for a refusal that says what to send.
fn wants(read: Read) -> String {
    match read {
        Read::Text => String::from("a string"),
        Read::Flag => String::from("true or false"),
        Read::Number => String::from("a number"),
        Read::Whole(least, most) => format!("a whole number from {least} to {most}"),
        Read::Size => String::from("`auto`, `fill` or a number above 0"),
        Read::Choice(_, list) => format!("one of {}", list.join(", ")),
        Read::Icon => format!("one of {}", SCREEN_ICONS.join(", ")),
        Read::Act => String::from("a string or an object with `text` and `icon`"),
        Read::Row => String::from("an object with `text` and, optionally, `meta`, `icon`, `selected`"),
        Read::Message => String::from("an object with `from` (me or them) and `text`"),
        Read::Many(one) => format!("a list of {}", plural(*one)),
        Read::Children => String::from("a list of elements"),
    }
}

fn plural(read: Read) -> String {
    match read {
        Read::Text => String::from("strings"),
        Read::Many(inner) => format!("lists of {}", plural(*inner)),
        Read::Act => String::from("strings or objects with `text` and `icon`"),
        Read::Row => String::from("objects with `text` and, optionally, `meta`, `icon`, `selected`"),
        Read::Message => String::from("objects with `from` (me or them) and `text`"),
        Read::Children => String::from("elements"),
        other => wants(other),
    }
}

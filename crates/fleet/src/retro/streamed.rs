//! A retro's items as the model writes them: the prefix of its JSON answer read for what has
//! settled. **Pure**, and `proposing::Settled::of`'s rule for a different shape: the whole
//! prefix every time, and a value counts only once it has ended.
//!
//! A Job's proposal is lines and ends a value at a newline. A retro is JSON, so a text ends at
//! its closing quote and nothing before it is read, which is what stops a title landing as
//! `Walks opened in the bro`. No `serde_json` here: bytes enter the process in `ipc`, and a
//! prefix is not a document.

use ipc::{LandsIn, PartialRetroItem, Whose};

/// What is open at this point of the prefix.
enum Open {
    /// An object, the key it has read, and whether the next string is a key.
    Object {
        key: Option<String>,
        wants_key: bool,
    },
    Array,
}

/// Where the reader is: the open containers, whether the array at depth two is the root's
/// `items`, and every item object begun, in order.
#[derive(Default)]
struct Reading {
    open: Vec<Open>,
    in_items: bool,
    begun: Vec<PartialRetroItem>,
}

impl Reading {
    /// A string just ended: a key, or the value of one.
    fn ended(&mut self, text: String) {
        // [root object, items array, item object]: the only place a field is read.
        let at_item = self.in_items && self.open.len() == 3;
        let Some(Open::Object { key, wants_key }) = self.open.last_mut() else {
            return;
        };
        if *wants_key {
            *key = Some(text);
            *wants_key = false;
            return;
        }
        let (true, Some(name), Some(item)) = (at_item, key.as_deref(), self.begun.last_mut())
        else {
            return;
        };
        let value = text.trim().to_string();
        match name {
            "title" => item.title = Some(value),
            "what" => item.what = Some(value),
            "fix" => item.fix = Some(value),
            "who" => item.who = Whose::from_wire(&value),
            "lands_in" => item.lands_in = LandsIn::from_wire(&value),
            _ => {}
        }
    }

    fn opened_object(&mut self) {
        if self.in_items && self.open.len() == 2 {
            self.begun.push(PartialRetroItem::default());
        }
        self.open.push(Open::Object {
            key: None,
            wants_key: true,
        });
    }

    fn opened_array(&mut self) {
        if let [Open::Object { key, .. }] = self.open.as_slice() {
            self.in_items = key.as_deref() == Some("items");
        }
        self.open.push(Open::Array);
    }

    fn comma(&mut self) {
        if let Some(Open::Object { key, wants_key }) = self.open.last_mut() {
            *key = None;
            *wants_key = true;
        }
    }
}

/// The items the answer has settled so far, in the order written. An item appears once one of
/// its fields has ended. **The model's items, before Fleet holds them to the record**, so the
/// kept retro can have fewer.
pub(crate) fn settled_items(written: &str) -> Vec<PartialRetroItem> {
    let mut reading = Reading::default();
    let mut began = false;
    let mut text: Option<String> = None;
    let mut chars = written.chars();
    while let Some(c) = chars.next() {
        if let Some(inside) = text.as_mut() {
            match c {
                '"' => {
                    let ended = text.take().unwrap_or_default();
                    reading.ended(ended);
                }
                '\\' => match chars.next() {
                    Some('n') => inside.push('\n'),
                    Some('t') => inside.push('\t'),
                    Some('r') => inside.push('\r'),
                    Some('u') => {
                        let hex: String = chars.by_ref().take(4).collect();
                        match u32::from_str_radix(&hex, 16) {
                            Ok(code) if hex.len() == 4 => {
                                inside.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                            }
                            // The escape itself is still arriving.
                            _ => break,
                        }
                    }
                    Some(other) => inside.push(other),
                    None => break,
                },
                c => inside.push(c),
            }
            continue;
        }
        match c {
            // Prose or a code fence ahead of the object is not read.
            _ if !began && c != '{' => {}
            '"' => text = Some(String::new()),
            '{' => {
                began = true;
                reading.opened_object();
            }
            '[' => reading.opened_array(),
            '}' | ']' => {
                reading.open.pop();
                if reading.open.is_empty() {
                    break;
                }
            }
            ',' => reading.comma(),
            _ => {}
        }
    }
    reading
        .begun
        .into_iter()
        .filter(|item| *item != PartialRetroItem::default())
        .collect()
}

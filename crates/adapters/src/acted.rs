//! What a Commander's act was on, read out of the call that took it.
//!
//! **The wire carries one line of argument per call**, so each act is reduced
//! here to the one figure a row can name it by: a cap's new number, the task
//! restarted. Nothing else of the body is
//! kept, and `CallDetail` bounds the line as it does any other.
//!
//! **Only the acts below are read, by operation.** A door call carries its path
//! segments at the top level and its fields under `body`, and other tools
//! carry a `body` or a `task_id` of their own; reading by name keeps their rows
//! as they were.

use serde::de::value::MapAccessDeserializer;
use serde::de::{IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// A string or a number as text, and anything else as nothing.
///
/// **Never an error.** These keys belong to every tool in the stream, and one
/// of them holding an object would otherwise make its whole line unreadable.
#[derive(Default)]
pub(crate) struct Said(Option<String>);

impl<'de> Deserialize<'de> for Said {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Said, D::Error> {
        struct Text;
        impl<'de> Visitor<'de> for Text {
            type Value = Said;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any value")
            }
            fn visit_str<E>(self, text: &str) -> Result<Said, E> {
                Ok(Said(Some(String::from(text))))
            }
            fn visit_u64<E>(self, number: u64) -> Result<Said, E> {
                Ok(Said(Some(number.to_string())))
            }
            fn visit_i64<E>(self, number: i64) -> Result<Said, E> {
                Ok(Said(Some(number.to_string())))
            }
            fn visit_bool<E>(self, _: bool) -> Result<Said, E> {
                Ok(Said(None))
            }
            fn visit_f64<E>(self, _: f64) -> Result<Said, E> {
                Ok(Said(None))
            }
            fn visit_unit<E>(self) -> Result<Said, E> {
                Ok(Said(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Said, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(Said(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Said, A::Error> {
                while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                Ok(Said(None))
            }
        }
        decoder.deserialize_any(Text)
    }
}

/// The fields of a door call's `body` that name what an act was on.
#[derive(Deserialize, Default)]
struct Fields {
    turn_cap: Option<Said>,
    cost_cap_micros: Option<Said>,
    title: Option<Said>,
    task: Option<Said>,
    group: Option<Said>,
    // `show_sketch`'s own fields. `title` is read above, and `scene` is held only as long as the call's
    // line is being read: what is made of it is decided by the operation, in [`sketch_call`].
    against: Option<Said>,
    scene: Option<ipc::RawScene>,
    // `show_file`'s path, as the agent gave it: the row names the file, and Fleet's receipt has the resolved one.
    path: Option<Said>,
}

/// A door call's `body`, or nothing where it is not an object.
///
/// **Another tool's `body` may be a string** (an issue's text, say), and that
/// must not cost the line.
#[derive(Default)]
pub(crate) struct Body(Fields);

impl<'de> Deserialize<'de> for Body {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Body, D::Error> {
        struct Object;
        impl<'de> Visitor<'de> for Object {
            type Value = Body;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("any value")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Body, A::Error> {
                Fields::deserialize(MapAccessDeserializer::new(map)).map(Body)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Body, A::Error> {
                while seq.next_element::<IgnoredAny>()?.is_some() {}
                Ok(Body::default())
            }
            fn visit_str<E>(self, _: &str) -> Result<Body, E> {
                Ok(Body::default())
            }
            fn visit_bool<E>(self, _: bool) -> Result<Body, E> {
                Ok(Body::default())
            }
            fn visit_u64<E>(self, _: u64) -> Result<Body, E> {
                Ok(Body::default())
            }
            fn visit_i64<E>(self, _: i64) -> Result<Body, E> {
                Ok(Body::default())
            }
            fn visit_f64<E>(self, _: f64) -> Result<Body, E> {
                Ok(Body::default())
            }
            fn visit_unit<E>(self) -> Result<Body, E> {
                Ok(Body::default())
            }
        }
        decoder.deserialize_any(Object)
    }
}

/// The arguments of one call that an act is read from.
pub(crate) struct Arguments<'a> {
    pub task_id: Option<&'a Said>,
    pub drone_id: Option<&'a Said>,
    pub body: Option<&'a Body>,
}

/// The figure one Commander act is named by, or `None` for an operation not
/// read here or a call that did not send it.
///
/// `operation` is the tool's name without the server that prefixes it. A cap
/// raise states the new ceiling, a restart or an edit the task, and a move the
/// task and where it went, or the group alone where a group moved.
pub(crate) fn figure(operation: &str, call: &Arguments) -> Option<String> {
    let fields = call.body.map(|body| &body.0);
    let said = |field: fn(&Fields) -> &Option<Said>| -> Option<&str> {
        fields.and_then(|fields| field(fields).as_ref()).and_then(|said| said.0.as_deref())
    };
    match operation {
        "raise_turn_cap" => said(|fields| &fields.turn_cap).map(String::from),
        "raise_cost_cap" => said(|fields| &fields.cost_cap_micros)
            .and_then(|micros| micros.parse::<u64>().ok())
            .map(dollars),
        "restart_task" | "edit_task" => call.task_id.and_then(|said| said.0.clone()),
        "add_task" => said(|fields| &fields.title).map(String::from),
        "drop_task" => said(|fields| &fields.task).map(String::from),
        "move_plan" => match (said(|fields| &fields.task), said(|fields| &fields.group)) {
            (Some(task), Some(group)) => Some(format!("{task} to {group}")),
            (None, Some(group)) => Some(String::from(group)),
            _ => None,
        },
        "show_file" => said(|fields| &fields.path).map(String::from),
        "redirect_one_drone" => call.drone_id.and_then(|said| said.0.clone()),
        _ => None,
    }
}

/// A `show_sketch` call's arguments as JSON text, whole, or `None` for any other operation and for a
/// call that sent no title or no scene.
///
/// **The one operation whose row carries its content**, because the row is what Bridge draws the sketch
/// from: the thread card reads `<tool> <this text>`. The door sent the fields under `body` and they
/// are unwrapped here, so the text is `{"title",…}` at the top level. The call was capped at 32 KiB by
/// the tool, so the text is as long as that and no more; `CallDetail` keeps it whole beside the
/// 200-character line a row shows. `session_id` is not carried: Fleet places the call by its connection.
pub(crate) fn sketch_call(operation: &str, call: &Arguments) -> Option<String> {
    if operation != "show_sketch" {
        return None;
    }
    let fields = call.body.map(|body| &body.0)?;
    let said = |field: &Option<Said>| field.as_ref().and_then(|said| said.0.clone());
    let show = ipc::ShowSketch {
        title: said(&fields.title)?,
        scene: fields.scene.clone()?,
        session_id: None,
        against: said(&fields.against),
    };
    ipc::encode(&show).ok()
}

/// A figure in millionths of a dollar, to the cent and without cents where
/// there are none: `$12`, `$12.50`.
fn dollars(micros: u64) -> String {
    let cents = micros.saturating_add(5_000) / 10_000;
    match cents % 100 {
        0 => format!("${}", cents / 100),
        rest => format!("${}.{rest:02}", cents / 100),
    }
}

//! `armada job`'s side of the door: the two JSON-RPC messages it sends and the answers it reads.
//!
//! **The door's own client**, as `armada mcp` is its relay. It lists with `tools/list` and calls with
//! `tools/call`, so an act added to the inventory is an act `armada job` offers with no change here.
//! The bytes are read in this crate and nowhere in `armada`, which holds no `serde_json`.

use serde_json::{json, Map, Value};

use crate::door::ASKS_A_PERSON;

/// One act the door lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Act {
    pub name: String,
    pub description: String,
    /// What the door reads as a path segment or a query name: a top-level argument.
    pub fields: Vec<Field>,
    /// The act is a command, so everything else it is given goes into its `body`.
    pub takes_body: bool,
}

/// One top-level argument of an act.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// What the door said to a call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The act's own answer.
    Done(String),
    /// Fleet's sentence for why it did not.
    Refused(String),
}

/// The `tools/list` message.
pub fn list_message() -> String {
    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }).to_string()
}

/// The acts a `tools/list` answer holds, in the order the door gave them.
///
/// **`ask_the_person` is left out**: it is the harness's permission tool, called by the CLI that
/// runs the agent, and an agent that called it by hand would be asking itself.
pub fn acts(answer: &[u8]) -> Result<Vec<Act>, String> {
    let message: Value = serde_json::from_slice(answer)
        .map_err(|why| format!("the door's list could not be read: {why}"))?;
    if let Some(error) = message.get("error") {
        return Err(failure(error));
    }
    let tools = message
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .ok_or_else(|| String::from("the door's answer to `tools/list` held no tools"))?;
    Ok(tools
        .iter()
        .filter_map(act_of)
        .filter(|act| act.name != ASKS_A_PERSON)
        .collect())
}

fn act_of(tool: &Value) -> Option<Act> {
    let name = tool.get("name")?.as_str()?.to_string();
    let description = tool
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let schema = tool.get("inputSchema");
    let required: Vec<&str> = schema
        .and_then(|schema| schema.get("required"))
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let properties = schema
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object);
    let mut fields = Vec::new();
    let mut takes_body = false;
    for (field, shape) in properties.into_iter().flatten() {
        if field == "body" {
            takes_body = true;
            continue;
        }
        fields.push(Field {
            name: field.clone(),
            description: shape
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            required: required.contains(&field.as_str()),
        });
    }
    Some(Act { name, description, fields, takes_body })
}

impl Act {
    /// The `tools/call` message for this act.
    ///
    /// `fields` are the `name=value` words and `body` the one JSON argument. A name this act's
    /// schema lists is a top-level argument; anything else goes into the body, which a query or a
    /// path-only act has none of, so it is refused with the names it does take.
    ///
    /// `session` is the Job's own id. **A Job Fleet hosts is placed by its process tree** whatever
    /// this says, and one in a terminal is named by it, so it is set where the act takes one at the
    /// top level and in the body of a command that does not name it.
    pub fn call_message(
        &self,
        fields: &[(String, String)],
        body: Option<&str>,
        session: Option<&str>,
    ) -> Result<String, String> {
        let mut arguments = Map::new();
        let mut inner = match body {
            None => Map::new(),
            Some(text) => match serde_json::from_str::<Value>(text) {
                Ok(Value::Object(fields)) => fields,
                Ok(_) => return Err(String::from("the body is JSON but not an object")),
                Err(why) => return Err(format!("the body is not JSON: {why}")),
            },
        };
        for (name, value) in fields {
            if self.fields.iter().any(|field| field.name == *name) {
                arguments.insert(name.clone(), Value::String(value.clone()));
            } else if self.takes_body {
                inner.insert(name.clone(), typed(value));
            } else {
                return Err(format!(
                    "`{name}` is not a field of `{}`. {}",
                    self.name,
                    self.takes_only()
                ));
            }
        }
        if !self.takes_body && !inner.is_empty() {
            return Err(format!("`{}` takes no body. {}", self.name, self.takes_only()));
        }
        if let Some(session) = session {
            if self.fields.iter().any(|field| field.name == "session_id") {
                arguments
                    .entry("session_id")
                    .or_insert_with(|| Value::String(session.to_string()));
            } else if self.takes_body {
                inner
                    .entry("session_id")
                    .or_insert_with(|| Value::String(session.to_string()));
            }
        }
        if self.takes_body {
            arguments.insert("body".to_string(), Value::Object(inner));
        }
        Ok(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": { "name": self.name, "arguments": arguments },
        })
        .to_string())
    }

    fn takes_only(&self) -> String {
        match self.fields.is_empty() {
            true => String::from("It takes none."),
            false => format!(
                "It takes {}.",
                self.fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// A `name=value` word's value as a body field: `true`, `false`, a whole number, or JSON that opens
/// with `{`, `[` or `"` is that value, and anything else is text. **Quote to force text**:
/// `title='"2024"'`.
fn typed(value: &str) -> Value {
    match value {
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }
    if let Ok(number) = value.parse::<i64>() {
        return Value::from(number);
    }
    if value.starts_with(['{', '[', '"']) {
        if let Ok(parsed) = serde_json::from_str::<Value>(value) {
            return parsed;
        }
    }
    Value::String(value.to_string())
}

/// What the door's answer to a `tools/call` says. A JSON-RPC error and a tool's `isError` are both
/// a refusal; the first line a person reads is Fleet's own sentence either way.
pub fn outcome(answer: &[u8]) -> Outcome {
    let message: Value = match serde_json::from_slice(answer) {
        Ok(message) => message,
        Err(why) => return Outcome::Refused(format!("the door's answer could not be read: {why}")),
    };
    if let Some(error) = message.get("error") {
        return Outcome::Refused(failure(error));
    }
    let Some(result) = message.get("result") else {
        return Outcome::Refused(String::from("the door's answer held no result"));
    };
    let text = result
        .get("content")
        .and_then(Value::as_array)
        .map(|blocks| {
            blocks
                .iter()
                .map(|block| match block.get("text").and_then(Value::as_str) {
                    Some(text) => text.to_string(),
                    None => String::from("[an image, which is not printed]"),
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    match result.get("isError").and_then(Value::as_bool) {
        Some(true) => Outcome::Refused(text),
        _ => Outcome::Done(text),
    }
}

fn failure(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("the door answered with an error and no message")
        .to_string()
}

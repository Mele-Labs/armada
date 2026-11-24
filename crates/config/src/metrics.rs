//! The `look.metrics:` block of a project's file: a list of recipes, each one call and a `jq` map. `docs/practices/protocol/projects.md`,
//! *Metrics*. **Read as the board is**: every fault is named with its key, and what a recipe says is kept as written.
//! Whether a recipe can run is the adapter's to say when it is tried, not this reader's.

use core_model::{MetricCall, MetricCredential, MetricRecipe};
use serde_yaml_ng::{Mapping, Value};

use crate::error::{Fault, Refusal};
use crate::yaml::{self, Table};

const RECIPE: &[&str] = &["id", "title", "call", "map", "every_secs", "credential"];
const CREDENTIAL: &[&str] = &["name", "how", "url"];
const CLI: &[&str] = &["transport", "argv", "env"];
const HTTP: &[&str] = &["transport", "method", "url", "headers", "body"];
const MCP: &[&str] = &["transport", "server", "env", "tool", "arguments"];
const TRANSPORTS: &[&str] = &["cli", "http", "mcp"];

/// The list at `key`. **An empty list is none**, and so is a file that wrote none.
pub(crate) fn read_metrics(key: &str, root: &Value, out: &mut Vec<Refusal>) -> Vec<MetricRecipe> {
    if matches!(root, Value::Sequence(items) if items.is_empty()) {
        return Vec::new();
    }
    let Some(items) = yaml::list(key, root, out) else { return Vec::new() };
    let mut recipes: Vec<MetricRecipe> = Vec::new();
    for (at, item) in items {
        let Some(recipe) = read_recipe(&at, item, out) else { continue };
        if recipes.iter().any(|held| held.id == recipe.id) {
            out.push(Refusal::new(format!("{at}.id"), Fault::NotInTheSchema { value: recipe.id.clone(), legal: &["an id no other recipe of the project has"] }));
            continue;
        }
        recipes.push(recipe);
    }
    recipes
}

fn read_recipe(key: &str, root: &Value, out: &mut Vec<Refusal>) -> Option<MetricRecipe> {
    let mut table = Table::open(key, root, out)?;
    let id = table.required("id", out).and_then(|value| yaml::text(&table.at("id"), value, out));
    let title = table.required("title", out).and_then(|value| yaml::text(&table.at("title"), value, out));
    let map = table.required("map", out).and_then(|value| yaml::text(&table.at("map"), value, out));
    let call_at = table.at("call");
    let call = table.required("call", out).and_then(|value| read_call(&call_at, value, out));
    let every_at = table.at("every_secs");
    let every_secs = table.optional("every_secs").and_then(|value| yaml::positive(&every_at, value, out));
    let credential_at = table.at("credential");
    let credential = table.optional("credential").and_then(|value| read_credential(&credential_at, value, out));
    table.close(RECIPE, out);
    Some(MetricRecipe { id: id?, title: title?, call: call?, map: map?, every_secs, credential })
}

fn read_credential(key: &str, root: &Value, out: &mut Vec<Refusal>) -> Option<MetricCredential> {
    let mut table = Table::open(key, root, out)?;
    let name = table.required("name", out).and_then(|value| yaml::text(&table.at("name"), value, out));
    let how = table.required("how", out).and_then(|value| yaml::text(&table.at("how"), value, out));
    let url = table.optional("url").and_then(|value| yaml::text(&table.at("url"), value, out));
    table.close(CREDENTIAL, out);
    Some(MetricCredential { name: name?, how: how?, url })
}

fn read_call(key: &str, root: &Value, out: &mut Vec<Refusal>) -> Option<MetricCall> {
    let mut table = Table::open(key, root, out)?;
    let transport_at = table.at("transport");
    let transport = table.required("transport", out).and_then(|value| yaml::text(&transport_at, value, out))?;
    let call = match transport.as_str() {
        "cli" => {
            let argv = words(&mut table, "argv", out);
            let env = pairs(&mut table, "env", out);
            table.close(CLI, out);
            MetricCall::Cli { argv: argv?, env }
        }
        "http" => {
            let method_at = table.at("method");
            let method = table.required("method", out).and_then(|value| yaml::text(&method_at, value, out));
            let url = table.required("url", out).and_then(|value| yaml::text(&table.at("url"), value, out));
            let headers = pairs(&mut table, "headers", out);
            let body = table.optional("body").and_then(|value| yaml::text(&table.at("body"), value, out));
            table.close(HTTP, out);
            MetricCall::Http { method: method?, url: url?, headers, body }
        }
        "mcp" => {
            let server = words(&mut table, "server", out);
            let env = pairs(&mut table, "env", out);
            let tool = table.required("tool", out).and_then(|value| yaml::text(&table.at("tool"), value, out));
            let arguments = table.optional("arguments").and_then(|value| yaml::text(&table.at("arguments"), value, out));
            table.close(MCP, out);
            MetricCall::Mcp { server: server?, env, tool: tool?, arguments: arguments.unwrap_or_else(|| "{}".to_string()) }
        }
        other => {
            out.push(Refusal::new(transport_at, Fault::NotInTheSchema { value: other.to_string(), legal: TRANSPORTS }));
            return None;
        }
    };
    Some(call)
}

/// The list of text at `name`, which the call needs.
fn words(table: &mut Table<'_>, name: &str, out: &mut Vec<Refusal>) -> Option<Vec<String>> {
    let at = table.at(name);
    let value = table.required(name, out)?;
    let items = yaml::list(&at, value, out)?;
    let words: Vec<Option<String>> = items.into_iter().map(|(item_at, item)| yaml::text(&item_at, item, out)).collect();
    words.into_iter().collect()
}

/// A map of text to text at `name`, in the order written. Absent is none.
fn pairs(table: &mut Table<'_>, name: &str, out: &mut Vec<Refusal>) -> Vec<(String, String)> {
    let at = table.at(name);
    let Some(value) = table.optional(name) else { return Vec::new() };
    let Some(inner) = Table::open(&at, value, out) else { return Vec::new() };
    inner
        .into_entries()
        .into_iter()
        .filter_map(|(each, value)| yaml::text(&format!("{at}.{each}"), value, out).map(|text| (each, text)))
        .collect()
}

/// The list as YAML, in the order held.
pub(crate) fn metrics_value(recipes: &[MetricRecipe]) -> Value {
    let text = |written: &str| Value::String(written.to_string());
    let map_of = |held: &[(String, String)]| {
        let mut written = Mapping::new();
        for (name, value) in held {
            written.insert(text(name), text(value));
        }
        Value::Mapping(written)
    };
    let list_of = |held: &[String]| Value::Sequence(held.iter().map(|one| text(one)).collect());
    Value::Sequence(
        recipes
            .iter()
            .map(|recipe| {
                let mut written = Mapping::new();
                written.insert(text("id"), text(&recipe.id));
                written.insert(text("title"), text(&recipe.title));
                let mut call = Mapping::new();
                match &recipe.call {
                    MetricCall::Cli { argv, env } => {
                        call.insert(text("transport"), text("cli"));
                        call.insert(text("argv"), list_of(argv));
                        if !env.is_empty() {
                            call.insert(text("env"), map_of(env));
                        }
                    }
                    MetricCall::Http { method, url, headers, body } => {
                        call.insert(text("transport"), text("http"));
                        call.insert(text("method"), text(method));
                        call.insert(text("url"), text(url));
                        if !headers.is_empty() {
                            call.insert(text("headers"), map_of(headers));
                        }
                        if let Some(body) = body {
                            call.insert(text("body"), text(body));
                        }
                    }
                    MetricCall::Mcp { server, env, tool, arguments } => {
                        call.insert(text("transport"), text("mcp"));
                        call.insert(text("server"), list_of(server));
                        if !env.is_empty() {
                            call.insert(text("env"), map_of(env));
                        }
                        call.insert(text("tool"), text(tool));
                        call.insert(text("arguments"), text(arguments));
                    }
                }
                written.insert(text("call"), Value::Mapping(call));
                written.insert(text("map"), text(&recipe.map));
                if let Some(every) = recipe.every_secs {
                    written.insert(text("every_secs"), Value::Number(every.into()));
                }
                if let Some(credential) = &recipe.credential {
                    let mut held = Mapping::new();
                    held.insert(text("name"), text(&credential.name));
                    held.insert(text("how"), text(&credential.how));
                    if let Some(url) = &credential.url {
                        held.insert(text("url"), text(url));
                    }
                    written.insert(text("credential"), Value::Mapping(held));
                }
                Value::Mapping(written)
            })
            .collect(),
    )
}

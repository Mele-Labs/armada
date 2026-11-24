//! A project's live metric, run with no model: one call as a tracker mod's call is run, the answer through the recipe's
//! `jq` map, and what the map prints read as a [`MetricAnswer`]. `docs/practices/protocol/projects.md`, *Metrics*.
//!
//! **The same machinery and the same bounds as a tracker's call**: 30 seconds, 4 MiB of answer, no shell, and every
//! refusal scrubbed of the token before it is said. What differs is where the token may go: `{credential}` is filled into
//! `env`, `arguments`, `headers` and `body`, and **never `argv`, `server` or `url`**, which a process list or a log would show.

use adapter_traits::Secret;
use ipc::{MetricAnswer, MetricCall, MetricRecipe};

use super::file::{placeholders_in, texts_of, url_problem, Transport, CREDENTIAL};
use super::running::{self, Fills};
use super::{answer_of, mapping};

/// The recipe's problems, or none where it can be run. **Nothing is run to find them.**
pub fn problem_with(recipe: &MetricRecipe) -> Option<String> {
    let id_ok = !recipe.id.is_empty()
        && recipe.id.len() <= 64
        && recipe.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !id_ok {
        return Some("a recipe's id is lower case letters, digits and dashes".to_string());
    }
    if recipe.title.trim().is_empty() || recipe.map.trim().is_empty() {
        return Some("a recipe has a title and a `map`".to_string());
    }
    if let Some(credential) = &recipe.credential {
        let name_ok = !credential.name.is_empty()
            && credential.name.len() <= 64
            && credential.name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        if !name_ok {
            return Some("a credential is named by letters, digits, dashes and underscores".to_string());
        }
    }
    let transport = transport_of(&recipe.call);
    let (here, away) = filled_and_not(&recipe.call);
    if here.iter().any(|one| placeholders_in(one).iter().any(|word| word != CREDENTIAL)) {
        return Some("`{credential}` is the only placeholder a metric call fills".to_string());
    }
    if away.iter().any(|one| placeholders_in(one).iter().any(|word| word == CREDENTIAL)) {
        return Some("`{credential}` is filled into `env`, `arguments`, `headers` and `body` only, never `argv`, `server` or `url`".to_string());
    }
    let uses_credential = texts_of(&transport).iter().any(|one| placeholders_in(one).iter().any(|word| word == CREDENTIAL));
    if uses_credential && recipe.credential.is_none() {
        return Some("the call uses `{credential}` and the recipe names no credential".to_string());
    }
    match &recipe.call {
        MetricCall::Cli { argv } => match argv.first() {
            Some(program) if !program.trim().is_empty() => None,
            _ => Some("a cli call names its program".to_string()),
        },
        MetricCall::Http { method, url, .. } => match method.to_ascii_uppercase().as_str() {
            "GET" | "POST" => url_problem(url).map(|why| format!("`url` {why}")),
            _ => Some("an http call is a `GET` or a `POST`".to_string()),
        },
        MetricCall::Mcp { server, tool, arguments, .. } => {
            if server.first().is_none_or(|program| program.trim().is_empty()) || tool.trim().is_empty() {
                return Some("an mcp call names its server and its tool".to_string());
            }
            match arguments.trim_start().starts_with('{') && ipc::same_json(arguments, arguments) {
                true => None,
                false => Some("an mcp call's `arguments` is a JSON object".to_string()),
            }
        }
    }
}

/// The texts a placeholder is filled into, and the ones it is not.
fn filled_and_not(call: &MetricCall) -> (Vec<&str>, Vec<&str>) {
    match call {
        MetricCall::Cli { argv } => (Vec::new(), argv.iter().map(String::as_str).collect()),
        MetricCall::Http { url, headers, body, .. } => {
            let mut here: Vec<&str> = headers.iter().map(|one| one.value.as_str()).collect();
            here.extend(body.as_deref());
            (here, vec![url.as_str()])
        }
        MetricCall::Mcp { server, env, arguments, .. } => {
            let mut here: Vec<&str> = env.iter().map(|one| one.value.as_str()).collect();
            here.push(arguments.as_str());
            (here, server.iter().map(String::as_str).collect())
        }
    }
}

fn transport_of(call: &MetricCall) -> Transport {
    match call {
        MetricCall::Cli { argv } => Transport::Cli { argv: argv.clone() },
        MetricCall::Http { method, url, headers, body } => Transport::Http {
            method: method.to_ascii_uppercase(),
            url: url.clone(),
            headers: headers.iter().map(|one| (one.name.clone(), one.value.clone())).collect(),
            body: body.clone(),
        },
        MetricCall::Mcp { server, env, tool, arguments } => Transport::Mcp {
            server: server.clone(),
            env: env.iter().map(|one| (one.name.clone(), one.value.clone())).collect(),
            tool: tool.clone(),
            arguments: arguments.clone(),
        },
    }
}

/// What `recipe` reads now, or why it did not. **Blocks** for as long as the call takes, so a caller on an async runtime
/// runs it on a blocking thread. The why is scrubbed of `token` and is a sentence for a person.
pub fn run_metric(recipe: &MetricRecipe, token: Option<&Secret<String>>) -> Result<MetricAnswer, String> {
    if let Some(problem) = problem_with(recipe) {
        return Err(problem);
    }
    let fills = Fills { credential: token, ..Fills::default() };
    let transport = transport_of(&recipe.call);
    let answer = answer_of(&transport, &fills, "reading the metric", running::WITHIN)
        .map_err(|not| not.said)?
        .unwrap_or_default();
    // Parenthesised, so a map is never read as a `jq` option.
    let jq = ["jq".to_string(), "-c".to_string(), format!("({}\n)", recipe.map)];
    let mapped = running::run(&jq, Some(&answer)).map_err(|why| fills.scrubbed(&why))?;
    if !mapped.succeeded {
        return Err(fills.scrubbed(&mapping::refusal_of(&mapped.stderr)));
    }
    ipc::decode::<MetricAnswer>("a metric's answer", &mapped.stdout)
        .map_err(|why| fills.scrubbed(&format!("the `map` did not print an answer of rows: {}", why.why)))
}

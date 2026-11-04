#![allow(dead_code)]
// Scratch harness: the real session_account.rs, mounted unchanged.
mod prompts {
    pub(crate) fn fill(template: &str, values: &[(&str, &str)]) -> String {
        let mut filled = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            filled.push_str(&rest[..open]);
            let from = &rest[open + 1..];
            let named = values.iter().find(|(name, _)| from.strip_prefix(name).is_some_and(|a| a.starts_with('}')));
            match named {
                Some((name, value)) => { filled.push_str(value); rest = &from[name.len() + 1..]; }
                None => { filled.push('{'); rest = from; }
            }
        }
        filled.push_str(rest);
        filled
    }
}
#[path = "../../crates/fleet/src/session_account.rs"]
mod session_account;

use session_account::*;
use std::fs;

fn items(path: &str) -> Vec<String> {
    if path == "-" { return vec![]; }
    fs::read_to_string(path).unwrap().split('\x1e').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}
fn groups(path: Option<&String>) -> Vec<(String, String)> {
    let Some(p) = path else { return vec![] };
    fs::read_to_string(p).unwrap().lines().filter_map(|l| l.split_once('\t')).map(|(a, b)| (a.to_string(), b.to_string())).collect()
}
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a[0].as_str() {
        "q-account" => {
            let t = TurnSoFar { message: fs::read_to_string(&a[1]).unwrap(), calls: items(&a[2]), said: items(&a[3]), groups: groups(a.get(4)) };
            print!("{}", t.question());
        }
        "q-sofar" => {
            let last = fs::read_to_string(&a[1]).ok().filter(|s| !s.trim().is_empty());
            let j = JobSoFar { last, message: Some(fs::read_to_string(&a[2]).unwrap()), done: items(&a[3]), said: items(&a[4]) };
            print!("{}", j.question());
        }
        "r-account" => {
            let reply = fs::read_to_string(&a[1]).unwrap();
            match read(&reply, &groups(a.get(2))) {
                None => println!("NONE"),
                Some(w) => {
                    println!("OK");
                    for d in &w.done { println!("done: {d}"); }
                    if let Some(n) = &w.now { println!("now: {n}"); }
                    if let Some(n) = &w.next { println!("next: {n}"); }
                    if let Some(g) = &w.group { println!("group: {g}"); }
                }
            }
        }
        "r-sofar" => {
            let reply = fs::read_to_string(&a[1]).unwrap();
            match read_so_far(&reply) { None => println!("NONE"), Some(t) => println!("OK\n{t}") }
        }
        _ => panic!("?"),
    }
}

//! What reaches what: the edges between pieces that the code names, and level one's links drawn from them and from the
//! modules' own dependencies. **Evidence only**: a route a client names that a server declares, a crate a file uses, an
//! import, a dependency a file reaches a store or an outside part by.

use std::collections::{BTreeMap, BTreeSet};

use ipc::{SystemLink, SystemPartKind};

use super::detect::{api_hosts, Detected, Role};
use super::model::{Edge, Node};
use super::repo::{extension_of, Repo};
use super::scanning::{declared, imported_words, literals, normal, path_of, refers};
use super::tables::{known, words};

/// What the pieces of one repository reach, and the routes its servers declare.
pub(super) struct Reaching {
    pub(super) edges: Vec<Edge>,
    pub(super) links: Vec<SystemLink>,
    pub(super) routes: Vec<(usize, String)>,
}

fn add(edges: &mut BTreeMap<(usize, Node, Option<String>), BTreeSet<String>>, file: &str, from: usize, to: Node, label: Option<String>) {
    edges.entry((from, to, label)).or_default().insert(file.to_string());
}

/// The parts that reach each module through libraries only, the module's own part counted.
fn reaching(repo: &Repo, det: &Detected) -> Vec<BTreeSet<usize>> {
    let mut by_module: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); repo.layout.modules.len()];
    for (at, part) in det.parts.iter().enumerate() {
        let mut queue: Vec<usize> = part.modules.clone();
        let mut seen: BTreeSet<usize> = queue.iter().copied().collect();
        while let Some(from) = queue.pop() {
            by_module[from].insert(at);
            for (_, to) in repo.layout.uses.iter().filter(|(one, _)| *one == from) {
                if matches!(det.roles[*to], Role::Lib | Role::Store) && seen.insert(*to) {
                    queue.push(*to);
                }
            }
        }
    }
    by_module
}

/// Of the parts that reach something, the services where any do, else all of them.
fn only_services(det: &Detected, parts: &BTreeSet<usize>) -> Vec<usize> {
    let services: Vec<usize> = parts.iter().copied().filter(|at| det.parts[*at].wire.kind == SystemPartKind::Service).collect();
    if services.is_empty() { parts.iter().copied().collect() } else { services }
}

pub(super) fn reach(repo: &Repo, det: &Detected, pieces: &[super::model::Piece]) -> Reaching {
    let mut edges: BTreeMap<(usize, Node, Option<String>), BTreeSet<String>> = BTreeMap::new();
    let piece_of = |path: &str| -> Option<usize> {
        pieces
            .iter()
            .enumerate()
            .filter_map(|(at, piece)| piece.paths.iter().filter(|prefix| super::model::holds(prefix, path)).map(|prefix| prefix.trim_matches('/').len()).max().map(|len| (len, at)))
            .max_by_key(|(len, _)| *len)
            .map(|(_, at)| at)
    };

    // Routes the code serves.
    let mut routes: Vec<(usize, String)> = Vec::new();
    let mut served: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for source in &repo.sources {
        let Some(piece) = piece_of(&source.path) else { continue };
        for route in declared(&source.path, &source.text) {
            let key = normal(&route);
            if key.split('/').filter(|segment| *segment != "*" && !segment.is_empty()).count() == 0 {
                continue;
            }
            routes.push((pieces[piece].part, route.clone()));
            served.entry(key).or_default().push((piece, route));
        }
    }

    // Module names as the code spells them.
    let idents: Vec<String> = repo.layout.modules.iter().map(|module| module.name.replace(['-', '.'], "_")).collect();
    let lib_piece = |module: usize| pieces.iter().position(|piece| piece.paths.first().is_some_and(|path| *path == format!("{}/", repo.layout.modules[module].root)));

    let outside_idents: Vec<Vec<String>> = det
        .outside
        .iter()
        .map(|found| found.rule.map(|rule| known().outside[rule].needs.iter().filter(|need| !need.ends_with('/')).map(|need| need.replace('-', "_")).collect()).unwrap_or_default())
        .collect();
    let outside_words: Vec<Vec<String>> = det.outside.iter().map(|found| found.rule.map(|rule| known().outside[rule].stems.clone()).unwrap_or_default()).collect();
    let outside_quoted: Vec<Vec<String>> = det.outside.iter().map(|found| found.rule.map(|rule| known().outside[rule].quoted.clone()).unwrap_or_default()).collect();

    // Pieces by the word an import names them by, per part.
    let mut by_word: BTreeMap<(usize, String), usize> = BTreeMap::new();
    for (at, piece) in pieces.iter().enumerate() {
        by_word.insert((piece.part, piece.name.clone()), at);
    }

    for source in &repo.sources {
        let Some(from) = piece_of(&source.path) else { continue };
        let own_part = pieces[from].part;
        let own_module = repo.layout.holding(&source.path);

        // A client naming a path a server declares.
        for literal in literals(&source.text) {
            let Some(path) = path_of(literal) else { continue };
            if let Some(there) = served.get(&normal(path)) {
                if let Some((to, route)) = there.iter().find(|(to, _)| pieces[*to].part != own_part) {
                    add(&mut edges, &source.path, from, Node::Piece(*to), Some(route.clone()));
                }
            }
        }

        // A file using another module's crate.
        if extension_of(&source.path) == "rs" {
            for (module, ident) in idents.iter().enumerate() {
                if Some(module) == own_module || ident.len() < 2 || !source.text.contains(&format!("{ident}::")) {
                    continue;
                }
                let used = source.text.match_indices(&format!("{ident}::")).any(|(at, _)| source.text[..at].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || matches!(c, '_' | ':'))));
                if !used {
                    continue;
                }
                match det.roles[module] {
                    Role::Store => {
                        if let Some(found) = det.stores.iter().find(|found| found.owners.contains(&module)) {
                            add(&mut edges, &source.path, from, Node::Part(found.part), None);
                        }
                    }
                    Role::Lib => {
                        if let Some(to) = lib_piece(module).filter(|to| *to != from) {
                            add(&mut edges, &source.path, from, Node::Piece(to), None);
                        }
                    }
                    Role::Entry | Role::Service | Role::Api => {
                        let Some(owner) = det.own[module] else { continue };
                        let after = source.text.match_indices(&format!("{ident}::")).filter_map(|(at, found)| source.text[at + found.len()..].split(|c: char| !(c.is_alphanumeric() || c == '_')).next()).find_map(|word| by_word.get(&(owner, word.to_string())));
                        if let Some(to) = after {
                            add(&mut edges, &source.path, from, Node::Piece(*to), None);
                        }
                    }
                    Role::Skip => {}
                }
            }
        }

        // An import of a sibling piece.
        for word in imported_words(&source.text) {
            if let Some(to) = by_word.get(&(own_part, word.to_string())).filter(|to| **to != from) {
                add(&mut edges, &source.path, from, Node::Piece(*to), None);
            }
        }

        // A store the file reaches by its dependency, or writes state files to.
        for found in &det.stores {
            let writes_files = found.idents.is_empty() && found.owners.is_empty() && det.parts[found.part].wire.paths.contains(&source.path);
            if writes_files || found.idents.iter().any(|ident| refers(&source.text, ident)) {
                add(&mut edges, &source.path, from, Node::Part(found.part), None);
            }
        }

        // Something outside the file names: a dependency, a file name, an address, a quoted word.
        let stem_words = words(source.path.rsplit('/').next().unwrap_or(&source.path));
        let hosts = api_hosts(&source.text);
        for (at, found) in det.outside.iter().enumerate() {
            let hit = outside_idents[at].iter().any(|ident| refers(&source.text, ident))
                || outside_words[at].iter().any(|stem| stem_words.contains(stem))
                || outside_quoted[at].iter().any(|word| source.text.contains(&format!("\"{word}\"")))
                || found.hosts.iter().any(|host| source.text.contains(host.as_str()) || hosts.contains(host));
            if hit {
                add(&mut edges, &source.path, from, Node::Part(found.part), None);
            }
        }
    }

    let edges: Vec<Edge> = edges.into_iter().map(|((from, to, label), files)| Edge { from, to, label, files: files.into_iter().collect() }).collect();
    let links = links_of(repo, det, pieces, &edges);
    Reaching { edges, links, routes }
}

/// Level one's links: the edges, folded to the parts they join, and what the modules' dependencies add.
/// One more link between two parts, with the label it carries.
fn fold(folded: &mut BTreeMap<(usize, usize), BTreeSet<Option<String>>>, from: usize, to: usize, label: Option<String>) {
    if from != to {
        folded.entry((from, to)).or_default().insert(label);
    }
}

fn links_of(repo: &Repo, det: &Detected, pieces: &[super::model::Piece], edges: &[Edge]) -> Vec<SystemLink> {
    let mut folded: BTreeMap<(usize, usize), BTreeSet<Option<String>>> = BTreeMap::new();
    for edge in edges {
        let from = pieces[edge.from].part;
        let to = match edge.to {
            Node::Piece(piece) => pieces[piece].part,
            Node::Part(part) => part,
        };
        fold(&mut folded, from, to, edge.label.clone());
    }

    let reached = reaching(repo, det);
    for (a, b) in &repo.layout.uses {
        let (Some(pa), Some(pb)) = (det.own[*a], det.own[*b]) else { continue };
        let (from, to) = (&det.parts[pa], &det.parts[pb]);
        if to.api && from.wire.kind == SystemPartKind::Service {
            fold(&mut folded, pb, pa, Some("served by".to_string()));
        } else if from.wire.kind == SystemPartKind::Entry && !from.api && to.wire.kind == SystemPartKind::Service {
            fold(&mut folded, pa, pb, None);
        }
    }
    for found in &det.stores {
        let mut parts: BTreeSet<usize> = BTreeSet::new();
        for module in found.owners.iter().chain(&found.users) {
            parts.extend(&reached[*module]);
        }
        for part in only_services(det, &parts) {
            fold(&mut folded, part, found.part, None);
        }
    }
    for found in &det.outside {
        let mut parts: BTreeSet<usize> = BTreeSet::new();
        for module in &found.users {
            parts.extend(&reached[*module]);
        }
        for part in only_services(det, &parts) {
            fold(&mut folded, part, found.part, None);
        }
    }

    // A web extension reads the pages its manifest matches.
    for found in det.outside.iter().filter(|found| matches!(&det.parts[found.part].wire.origin, ipc::PartOrigin::Detected { from } if from == "manifest.json")) {
        for (at, part) in det.parts.iter().enumerate() {
            if matches!(&part.wire.origin, ipc::PartOrigin::Detected { from } if from == "manifest.json") && part.wire.kind == SystemPartKind::Entry {
                fold(&mut folded, at, found.part, Some("reads pages".to_string()));
            }
        }
    }

    // A client that names no server is still a client of the one there is.
    let services: Vec<usize> = det.parts.iter().enumerate().filter(|(_, part)| part.wire.kind == SystemPartKind::Service).map(|(at, _)| at).collect();
    if let [only] = services[..] {
        for (at, part) in det.parts.iter().enumerate() {
            let talks = folded.keys().any(|(from, to)| *from == at && det.parts[*to].wire.kind != SystemPartKind::Outside && det.parts[*to].wire.kind != SystemPartKind::Store);
            if part.wire.kind == SystemPartKind::Entry && !part.api && !talks && part.wire.paths.iter().all(|path| path != "./") && !is_command(part) {
                fold(&mut folded, at, only, None);
            }
        }
        let keeps = folded.keys().any(|(from, to)| *from == only && det.parts[*to].wire.kind == SystemPartKind::Store);
        let stores: Vec<usize> = det.parts.iter().enumerate().filter(|(_, part)| part.wire.kind == SystemPartKind::Store).map(|(at, _)| at).collect();
        if let (false, [store]) = (keeps, &stores[..]) {
            fold(&mut folded, only, *store, None);
        }
    }

    // A server another part serves answers for it.
    let serving: BTreeMap<usize, usize> = folded.iter().filter(|(_, labels)| labels.contains(&Some("served by".to_string()))).map(|((from, to), _)| (*from, *to)).collect();
    let mut links: BTreeMap<(usize, usize), BTreeSet<Option<String>>> = BTreeMap::new();
    for ((from, to), labels) in folded {
        // A service naming the routes it serves is not a client of them.
        if det.parts[to].api && serving.get(&to) == Some(&from) {
            continue;
        }
        let target = match serving.get(&to) {
            Some(service) if det.parts[to].api && *service != from && det.parts[from].wire.kind == SystemPartKind::Entry => *service,
            _ => to,
        };
        links.entry((from, target)).or_default().extend(labels);
    }
    links
        .into_iter()
        .map(|((from, to), labels)| {
            let named: Vec<String> = labels.into_iter().flatten().collect();
            SystemLink { from: det.parts[from].wire.id.clone(), to: det.parts[to].wire.id.clone(), label: (named.len() == 1).then(|| named[0].clone()), step: None }
        })
        .collect()
}

fn is_command(part: &super::model::Part) -> bool {
    part.what == "command"
}

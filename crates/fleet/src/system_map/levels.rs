//! The three levels of the map, built from the repositories read: the parts, one part's pieces, one feature's path.

use std::sync::Arc;

use ipc::{PartOrigin, PathStop, SystemFeature, SystemLink, SystemMap, SystemPart, SystemPartKind};
use store::KeptFeature;

use super::model::{holds, Mapped, Node};
use crate::feature_library::mapping::owns;

/// The most files one stop of a path lists.
const MOST_FILES: usize = 40;

/// A part the librarian added, as the map draws it until he keeps it, and after.
#[derive(Clone, Debug)]
pub(super) struct Addition {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) kind: SystemPartKind,
    pub(super) summary: String,
    pub(super) why: String,
    pub(super) paths: Vec<String>,
    pub(super) kept: bool,
    pub(super) manifest: String,
}

impl Addition {
    pub(super) fn part(&self, several: bool) -> SystemPart {
        SystemPart {
            id: format!("added-{}", self.id),
            name: self.name.clone(),
            kind: self.kind,
            summary: self.summary.clone(),
            within: None,
            manifest_id: several.then(|| ipc::ManifestId::carried(&self.manifest)),
            paths: self.paths.clone(),
            zooms: false,
            origin: if self.kept { PartOrigin::Detected { from: "librarian".to_string() } } else { PartOrigin::Librarian { addition: self.id.clone(), why: self.why.clone() } },
        }
    }
}

/// What the levels are built from.
pub(super) struct Ground<'a> {
    pub(super) project: &'a str,
    pub(super) maps: &'a [Arc<Mapped>],
    pub(super) features: &'a [KeptFeature],
    pub(super) filed: &'a (dyn Fn(&str, &str) -> bool + Sync),
    pub(super) additions: &'a [Addition],
    pub(super) librarian_at: Option<ipc::Instant>,
}

/// A part found by its id: a detected one, or one the librarian added.
pub(super) enum Found<'a> {
    Detected { repo: usize, part: usize },
    Added(&'a Addition),
}

impl Ground<'_> {
    fn several(&self) -> bool {
        self.maps.len() > 1
    }

    /// A part's id on the wire: its own, unless another repository has a part of the same id.
    pub(super) fn part_id(&self, repo: usize, part: usize) -> String {
        let local = &self.maps[repo].parts[part].wire.id;
        let clash = self.maps.iter().enumerate().any(|(other, map)| other != repo && map.parts.iter().any(|one| one.wire.id == *local));
        if clash { format!("{local}@{}", self.maps[repo].manifest) } else { local.clone() }
    }

    fn piece_id(&self, repo: usize, piece: usize) -> String {
        let map = &self.maps[repo];
        let item = &map.pieces[piece];
        let rest = item.id.split_once('/').map_or(item.id.as_str(), |(_, rest)| rest);
        format!("{}/{rest}", self.part_id(repo, item.part))
    }

    pub(super) fn find(&self, id: &str) -> Option<Found<'_>> {
        for (repo, map) in self.maps.iter().enumerate() {
            if let Some(part) = (0..map.parts.len()).find(|part| self.part_id(repo, *part) == id) {
                return Some(Found::Detected { repo, part });
            }
        }
        self.additions.iter().find(|one| format!("added-{}", one.id) == id).map(Found::Added)
    }

    /// A part or piece id found as a piece: its repository, its index.
    fn find_piece(&self, id: &str) -> Option<(usize, usize)> {
        self.maps.iter().enumerate().find_map(|(repo, map)| (0..map.pieces.len()).find(|piece| self.piece_id(repo, *piece) == id).map(|piece| (repo, piece)))
    }

    fn wire(&self, repo: usize, part: usize) -> SystemPart {
        let mut wire = self.maps[repo].parts[part].wire.clone();
        wire.id = self.part_id(repo, part);
        wire.manifest_id = self.several().then(|| ipc::ManifestId::carried(&self.maps[repo].manifest));
        wire
    }

    fn piece_wire(&self, repo: usize, piece: usize) -> SystemPart {
        let map = &self.maps[repo];
        let item = &map.pieces[piece];
        SystemPart {
            id: self.piece_id(repo, piece),
            name: item.name.clone(),
            kind: SystemPartKind::Piece,
            summary: item.summary.clone(),
            within: Some(self.part_id(repo, item.part)),
            manifest_id: self.several().then(|| ipc::ManifestId::carried(&map.manifest)),
            paths: item.paths.clone(),
            zooms: false,
            origin: map.parts[item.part].wire.origin.clone(),
        }
    }

    /// Whether `feature` owns files under any of `paths` in the repository `manifest`.
    pub(super) fn feature_in(&self, feature: &KeptFeature, manifest: &str, paths: &[String]) -> bool {
        if !feature.manifest_id.is_empty() && feature.manifest_id != manifest {
            return false;
        }
        feature.paths.iter().any(|own| paths.iter().any(|path| holds(path, own.trim_matches('/')) || owns(own, path.trim_end_matches('/')) || path.trim_matches('/') == "."))
    }

    pub(super) fn features_over(&self, held: &[(usize, String, Vec<String>)]) -> Vec<SystemFeature> {
        self.features
            .iter()
            .filter_map(|feature| {
                let parts: Vec<String> = held.iter().filter(|(repo, _, paths)| self.feature_in(feature, &self.maps[*repo].manifest, paths)).map(|(_, id, _)| id.clone()).collect();
                (!parts.is_empty()).then(|| SystemFeature { slug: feature.slug.clone(), title: feature.title.clone(), parts })
            })
            .collect()
    }

    /// Every feature of the project, each with the level-one parts its files fall in.
    pub(super) fn all_features(&self) -> Vec<SystemFeature> {
        let mut held: Vec<(usize, String, Vec<String>)> = Vec::new();
        for (repo, map) in self.maps.iter().enumerate() {
            held.extend(map.parts.iter().enumerate().map(|(part, one)| (repo, self.part_id(repo, part), one.wire.paths.clone())));
        }
        for added in self.additions {
            let manifest = self.maps.iter().position(|map| map.manifest == added.manifest).unwrap_or(0);
            held.push((manifest, format!("added-{}", added.id), added.paths.clone()));
        }
        self.features
            .iter()
            .map(|feature| SystemFeature {
                slug: feature.slug.clone(),
                title: feature.title.clone(),
                parts: held.iter().filter(|(repo, _, paths)| self.feature_in(feature, &self.maps[*repo].manifest, paths)).map(|(_, id, _)| id.clone()).collect(),
            })
            .collect()
    }

    /// Whether any part that draws a screen holds `path` of the repository `repo`.
    pub(super) fn draws_screen_at(&self, repo: usize, path: &str) -> bool {
        self.maps[repo].parts.iter().any(|part| part.screen && part.wire.paths.iter().any(|prefix| holds(prefix, path)))
    }

    /// Level one.
    pub(super) fn level_one(&self) -> SystemMap {
        let mut parts = vec![SystemPart {
            id: "people".to_string(),
            name: "You".to_string(),
            kind: SystemPartKind::Person,
            summary: "Whoever uses it.".to_string(),
            within: None,
            manifest_id: None,
            paths: Vec::new(),
            zooms: false,
            origin: PartOrigin::Detected { from: "always".to_string() },
        }];
        let mut links: Vec<SystemLink> = Vec::new();
        let mut held: Vec<(usize, String, Vec<String>)> = Vec::new();
        for (repo, map) in self.maps.iter().enumerate() {
            for (part, one) in map.parts.iter().enumerate() {
                let id = self.part_id(repo, part);
                if one.wire.kind == SystemPartKind::Entry {
                    links.push(SystemLink { from: "people".to_string(), to: id.clone(), label: None, step: None });
                }
                held.push((repo, id, one.wire.paths.clone()));
                parts.push(self.wire(repo, part));
            }
            links.extend(map.links.iter().map(|link| {
                let id_of = |local: &str| map.part_at(local).map_or_else(|| local.to_string(), |part| self.part_id(repo, part));
                SystemLink { from: id_of(&link.from), to: id_of(&link.to), label: link.label.clone(), step: None }
            }));
        }
        for added in self.additions {
            let manifest = self.maps.iter().position(|map| map.manifest == added.manifest).unwrap_or(0);
            held.push((manifest, format!("added-{}", added.id), added.paths.clone()));
            parts.push(added.part(self.several()));
        }
        SystemMap {
            project: self.project.to_string(),
            part: None,
            feature: None,
            features: self.features_over(&held),
            parts,
            links,
            stops: Vec::new(),
            librarian_at: self.librarian_at.clone(),
        }
    }

    /// Level two: the pieces of `part`, the pieces and parts they reach, and the links between them.
    pub(super) fn level_two(&self, repo: usize, part: usize) -> SystemMap {
        let map = &self.maps[repo];
        let own: Vec<usize> = (0..map.pieces.len()).filter(|piece| map.pieces[*piece].part == part).collect();
        let mut parts: Vec<SystemPart> = own.iter().map(|piece| self.piece_wire(repo, *piece)).collect();
        let mut links: Vec<SystemLink> = Vec::new();
        let mut held: Vec<(usize, String, Vec<String>)> = own.iter().map(|piece| (repo, self.piece_id(repo, *piece), map.pieces[*piece].paths.clone())).collect();
        let mut seen: Vec<String> = parts.iter().map(|one| one.id.clone()).collect();
        for edge in map.edges.iter().filter(|edge| own.contains(&edge.from)) {
            let (to, wire) = match edge.to {
                Node::Piece(piece) => (self.piece_id(repo, piece), (!own.contains(&piece)).then(|| (self.piece_wire(repo, piece), map.pieces[piece].paths.clone()))),
                Node::Part(other) => (self.part_id(repo, other), Some((self.wire(repo, other), map.parts[other].wire.paths.clone()))),
            };
            if let Some((wire, paths)) = wire.filter(|(wire, _)| !seen.contains(&wire.id)) {
                seen.push(wire.id.clone());
                held.push((repo, wire.id.clone(), paths));
                parts.push(wire);
            }
            let link = SystemLink { from: self.piece_id(repo, edge.from), to, label: edge.label.clone(), step: None };
            if !links.contains(&link) {
                links.push(link);
            }
        }
        SystemMap {
            project: self.project.to_string(),
            part: Some(self.part_id(repo, part)),
            feature: None,
            features: self.features_over(&held),
            parts,
            links,
            stops: Vec::new(),
            librarian_at: self.librarian_at.clone(),
        }
    }

    /// Level three: the files `feature` owns, grouped by the part they fall in and numbered in level one's order.
    pub(super) fn level_three(&self, feature: &KeptFeature) -> SystemMap {
        let mut stops: Vec<(u8, String, usize, usize, Vec<String>)> = Vec::new();
        for (repo, map) in self.maps.iter().enumerate() {
            let mut by_part: std::collections::BTreeMap<usize, Vec<String>> = std::collections::BTreeMap::new();
            let candidates = map.files.iter().map(|(file, _)| file).filter(|file| {
                (feature.manifest_id.is_empty() || feature.manifest_id == map.manifest) && (feature.paths.iter().any(|own| owns(own, file)) || (self.filed)(&feature.id, file))
            });
            for file in candidates {
                if let Some(part) = map.part_of(file) {
                    by_part.entry(part).or_default().push(file.clone());
                }
            }
            for (part, mut files) in by_part {
                files.sort();
                let rank = match map.parts[part].wire.kind {
                    SystemPartKind::Person => 0,
                    SystemPartKind::Entry => 1,
                    SystemPartKind::Service => 2,
                    SystemPartKind::Store => 3,
                    _ => 4,
                };
                stops.push((rank, self.part_id(repo, part), repo, part, files));
            }
        }
        stops.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut parts = Vec::new();
        let mut path_stops = Vec::new();
        let mut links = Vec::new();
        for (at, (_, id, repo, part, mut files)) in stops.into_iter().enumerate() {
            files.truncate(MOST_FILES);
            let step = u32::try_from(at + 1).unwrap_or(u32::MAX);
            if let Some(previous) = path_stops.last().map(|one: &PathStop| one.part.clone()) {
                links.push(SystemLink { from: previous, to: id.clone(), label: None, step: Some(step - 1) });
            }
            parts.push(self.wire(repo, part));
            path_stops.push(PathStop { step, part: id, files });
        }
        // Level one's own links name what each step does where it has a name for it.
        let level_one = self.level_one().links;
        for link in &mut links {
            if link.label.is_none() {
                link.label = level_one.iter().find(|one| one.from == link.from && one.to == link.to).and_then(|one| one.label.clone());
            }
        }
        SystemMap {
            project: self.project.to_string(),
            part: None,
            feature: Some(feature.slug.clone()),
            features: vec![SystemFeature { slug: feature.slug.clone(), title: feature.title.clone(), parts: path_stops.iter().map(|stop| stop.part.clone()).collect() }],
            parts,
            links,
            stops: path_stops,
            librarian_at: self.librarian_at.clone(),
        }
    }

    /// What a part, a piece or a librarian's addition is called, where its code is and whether it draws a screen.
    pub(super) fn describe(&self, id: &str) -> Option<Described> {
        if let Some((repo, piece)) = self.find_piece(id) {
            let (map, item) = (&self.maps[repo], &self.maps[repo].pieces[piece]);
            return Some(Described { repo, manifest: map.manifest.clone(), name: item.name.clone(), screen: map.parts[item.part].screen, part: Some(item.part), paths: item.paths.clone() });
        }
        match self.find(id)? {
            Found::Detected { repo, part } => {
                let (map, one) = (&self.maps[repo], &self.maps[repo].parts[part]);
                Some(Described { repo, manifest: map.manifest.clone(), name: one.wire.name.clone(), screen: one.screen, part: Some(part), paths: one.wire.paths.clone() })
            }
            Found::Added(added) => {
                let repo = self.maps.iter().position(|map| map.manifest == added.manifest).unwrap_or(0);
                Some(Described { repo, manifest: added.manifest.clone(), name: added.name.clone(), screen: false, part: None, paths: added.paths.clone() })
            }
        }
    }
}

/// What [`Ground::describe`] answers.
pub(super) struct Described {
    pub(super) repo: usize,
    pub(super) manifest: String,
    pub(super) name: String,
    pub(super) screen: bool,
    pub(super) part: Option<usize>,
    pub(super) paths: Vec<String>,
}

//! The Code section's tree: the folders and files a part's code starts at, shut at their roots, a chain of single folders folded
//! into one row as an editor folds it, and a folder filled from what Fleet lists when it is opened. Nothing here touches the
//! screen or the wire.

use std::collections::{BTreeMap, HashMap};

/// What Fleet said of one folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listing {
    Reading,
    Names(Vec<String>),
    Failed(String),
}

/// One row of the tree as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The folder's or file's path from the repository, with no slash at its end.
    pub path: String,
    /// What the row says: a folded chain reads `crates/bridge-native/src`.
    pub words: String,
    pub dir: bool,
    pub depth: usize,
    pub open: bool,
    /// The folder is open and Fleet has not listed it yet.
    pub reading: bool,
    pub failed: Option<String>,
}

#[derive(Default)]
struct Trie {
    kids: BTreeMap<String, Trie>,
    path: String,
    dir: bool,
    /// A path the part named is where it ends: shut until he opens it, and filled from Fleet's listing.
    start: bool,
}

fn trie_of(code: &[String]) -> Trie {
    let mut root = Trie { dir: true, ..Trie::default() };
    for raw in code {
        let named_dir = raw.ends_with('/');
        let segments: Vec<&str> = raw.trim_end_matches('/').split('/').filter(|part| !part.is_empty()).collect();
        let mut at = &mut root;
        let mut path = String::new();
        for (index, segment) in segments.iter().enumerate() {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(segment);
            let last = index + 1 == segments.len();
            let here = at.kids.entry((*segment).to_string()).or_default();
            here.path = path.clone();
            here.dir |= !last || named_dir;
            at = here;
        }
        at.start = true;
    }
    root
}

/// The rows to draw. `toggled` holds the folders he opened or shut by hand, and `listed` what Fleet said of the folders asked.
pub fn rows(code: &[String], toggled: &HashMap<String, bool>, listed: &HashMap<String, Listing>) -> Vec<Row> {
    let root = trie_of(code);
    let mut out = Vec::new();
    for (name, kid) in sorted(&root.kids) {
        emit(name.clone(), kid, 0, toggled, listed, &mut out);
    }
    out
}

fn sorted(kids: &BTreeMap<String, Trie>) -> Vec<(&String, &Trie)> {
    let mut all: Vec<(&String, &Trie)> = kids.iter().collect();
    all.sort_by(|a, b| b.1.dir.cmp(&a.1.dir).then_with(|| a.0.cmp(b.0)));
    all
}

fn emit(mut words: String, mut node: &Trie, depth: usize, toggled: &HashMap<String, bool>, listed: &HashMap<String, Listing>, out: &mut Vec<Row>) {
    // A chain of folders with one folder in each is one row.
    while node.dir && !node.start && node.kids.len() == 1 {
        let Some((name, only)) = node.kids.iter().next() else { break };
        if !only.dir {
            break;
        }
        words.push('/');
        words.push_str(name);
        node = only;
    }
    if !node.dir {
        out.push(Row { path: node.path.clone(), words, dir: false, depth, open: false, reading: false, failed: None });
        return;
    }
    // A folder the part's paths run through is open; one they end at is shut until he opens it.
    let known = !node.kids.is_empty() && !node.start;
    let open = toggled.get(&node.path).copied().unwrap_or(known);
    let asked = if open && !known { listed.get(&node.path).or(Some(&Listing::Reading)) } else { None };
    out.push(Row {
        path: node.path.clone(),
        words,
        dir: true,
        depth,
        open,
        reading: matches!(asked, Some(Listing::Reading)),
        failed: match asked {
            Some(Listing::Failed(why)) => Some(why.clone()),
            _ => None,
        },
    });
    if !open {
        return;
    }
    if known {
        for (name, kid) in sorted(&node.kids) {
            emit(name.clone(), kid, depth + 1, toggled, listed, out);
        }
    } else if let Some(Listing::Names(names)) = asked {
        // Folders first, each in name order, whatever order Fleet listed them in.
        let mut names: Vec<&String> = names.iter().collect();
        names.sort_by(|a, b| b.ends_with('/').cmp(&a.ends_with('/')).then_with(|| a.cmp(b)));
        for name in names {
            let dir = name.ends_with('/');
            let name = name.trim_end_matches('/');
            let path = format!("{}/{name}", node.path);
            if dir {
                // A listed folder stands shut, with no folding: what is inside it is not known until it is asked for.
                let shut = Trie { dir: true, start: true, path: path.clone(), ..Trie::default() };
                emit(name.to_string(), &shut, depth + 1, toggled, listed, out);
            } else {
                out.push(Row { path, words: name.to_string(), dir: false, depth: depth + 1, open: false, reading: false, failed: None });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(rows: &[Row]) -> Vec<(usize, String, bool)> {
        rows.iter().map(|row| (row.depth, row.words.clone(), row.open)).collect()
    }

    fn code(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    #[test]
    fn a_single_folder_chain_is_one_row_and_its_end_starts_shut() {
        let rows = rows(&code(&["crates/bridge-native/src/project_board/"]), &HashMap::new(), &HashMap::new());
        assert_eq!(words(&rows), [(0, "crates/bridge-native/src/project_board".to_string(), false)]);
        assert!(rows[0].dir && !rows[0].reading, "shut, so nothing is asked for yet");
    }

    #[test]
    fn a_folder_two_paths_run_through_is_open_and_each_path_below_it_is_shut() {
        let rows = rows(&code(&["crates/bridge-native/src/project_board/", "crates/fleet/src/project_board.rs"]), &HashMap::new(), &HashMap::new());
        assert_eq!(
            words(&rows),
            [
                (0, "crates".to_string(), true),
                (1, "bridge-native/src/project_board".to_string(), false),
                (1, "fleet/src".to_string(), true),
                (2, "project_board.rs".to_string(), false),
            ],
            "the shared folder is open, a folded folder a path ends at is shut, and a file stands under its folded folder"
        );
        assert!(!rows[3].dir, "a path with no slash at its end and nothing under it is a file");
    }

    #[test]
    fn folders_stand_before_files_and_each_in_name_order() {
        let rows = rows(&code(&["a.py", "src/", "backend/", "z.md"]), &HashMap::new(), &HashMap::new());
        let names: Vec<&str> = rows.iter().map(|row| row.words.as_str()).collect();
        assert_eq!(names, ["backend", "src", "a.py", "z.md"]);
    }

    #[test]
    fn opening_a_shut_folder_waits_for_its_listing_and_fills_from_it_once_it_comes() {
        let paths = code(&["frontend/"]);
        let mut toggled = HashMap::new();
        let shut = rows(&paths, &toggled, &HashMap::new());
        assert!(shut[0].dir && !shut[0].open);
        toggled.insert("frontend".to_string(), true);
        let reading = rows(&paths, &toggled, &HashMap::new());
        assert!(reading[0].open && reading[0].reading, "open and waiting: a placeholder draws");
        let listed = HashMap::from([("frontend".to_string(), Listing::Names(vec!["src/".into(), "public/".into(), "package.json".into()]))]);
        let filled = rows(&paths, &toggled, &listed);
        assert_eq!(
            words(&filled),
            [(0, "frontend".to_string(), true), (1, "public".to_string(), false), (1, "src".to_string(), false), (1, "package.json".to_string(), false)],
            "folders first in name order, and each stands shut"
        );
        assert_eq!(filled[2].path, "frontend/src");
    }

    #[test]
    fn a_listing_that_failed_says_so_on_the_folder_and_a_shut_folder_hides_what_it_holds() {
        let paths = code(&["frontend/"]);
        let toggled = HashMap::from([("frontend".to_string(), true)]);
        let listed = HashMap::from([("frontend".to_string(), Listing::Failed("not served".into()))]);
        let rows_failed = rows(&paths, &toggled, &listed);
        assert_eq!(rows_failed[0].failed.as_deref(), Some("not served"));
        let toggled = HashMap::from([("frontend".to_string(), false)]);
        assert_eq!(rows(&paths, &toggled, &listed).len(), 1);
    }

    #[test]
    fn a_shared_folder_can_be_shut_by_hand() {
        let paths = code(&["crates/a/", "crates/b/"]);
        let toggled = HashMap::from([("crates".to_string(), false)]);
        let rows = rows(&paths, &toggled, &HashMap::new());
        assert_eq!(words(&rows), [(0, "crates".to_string(), false)]);
    }
}

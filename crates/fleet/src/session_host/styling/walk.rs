//! Finding the stylesheets nearest the code a Job works on. **A function of a folder and some
//! paths, reading only the disk**, so it is tested on temporary folders and means the same in every
//! repository. The steps are `.armada/documents/hi-fi-sketches.html`, *How Fleet finds the styles*.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ipc::{read_package_manifest, PackageManifest, StyleKind, StyleSource, StyleSources, StyleToken};

use super::sheet::{self, find_ci, SHEET_AT_MOST};
use super::tokens::{tokens_in, visual_count};

/// Folders no search enters: what a package manager or a build leaves, and other people's code.
const SKIPPED: [&str; 10] =
    ["node_modules", ".git", "target", "vendor", "third_party", "bower_components", ".venv", "venv", "__pycache__", "Pods"];
/// Most directory entries one walk visits, so a huge checkout costs a bounded read.
const VISITS_AT_MOST: usize = 20_000;
/// How deep a search for packages or theme files goes.
const DEPTH_AT_MOST: usize = 5;
/// Most plain stylesheets a search of the whole checkout offers.
const SEARCHED_AT_MOST: usize = 5;
/// Most stylesheets beside a file that are offered.
const BESIDE_AT_MOST: usize = 8;
/// Most bytes of an answer's tokens, by their names and values.
const TOKEN_BYTES_AT_MOST: usize = 48 * 1024;
/// Most packages an answer lists where it will not guess.
const PACKAGES_AT_MOST: usize = 60;
/// What a file with a style may end in.
const STYLE_EXTENSIONS: [&str; 4] = ["css", "scss", "sass", "less"];
/// What the Job's own work is looked at through: a file that is not one of these has no look.
const UI_EXTENSIONS: [&str; 22] = [
    "css", "scss", "sass", "less", "tsx", "jsx", "ts", "js", "mjs", "vue", "svelte", "html", "htm", "erb", "haml", "slim",
    "njk", "twig", "php", "astro", "mdx", "liquid",
];
/// The names a package's own stylesheet is conventionally given, best first.
const ENTRY_NAMES: [&str; 7] = ["index", "main", "styles", "style", "global", "globals", "app"];
const TAILWIND_PACKAGES: [&str; 4] = ["tailwindcss", "@tailwindcss/vite", "@tailwindcss/postcss", "@tailwindcss/cli"];

/// A file the walk found, by its path from the checkout's root.
struct Found {
    path: String,
    kind: StyleKind,
}

struct Package {
    /// From the checkout's root. Empty for the root itself.
    dir: String,
    manifest: PackageManifest,
}

/// The stylesheets nearest `named`, or else the first of `tiers` with a file in it. `tiers` are
/// the Job's changed files, then those it read, then those its plan names. `root` is the
/// checkout's canonical path.
pub(crate) fn sources_near(root: &Path, named: Option<&str>, tiers: &[Vec<String>]) -> StyleSources {
    let packages = packages_of(root);
    let start = match named {
        Some(path) => Some(path.to_string()),
        None => tiers.iter().find_map(|tier| tier.iter().find(|path| is_ui(root, path))).cloned(),
    };
    let listed = || packages.iter().map(|one| if one.dir.is_empty() { String::from(".") } else { one.dir.clone() });
    if start.is_none() && packages.len() > 1 {
        return StyleSources { sources: Vec::new(), start: None, packages: listed().take(PACKAGES_AT_MOST).collect() };
    }
    let mut found = Vec::new();
    let package = start.as_deref().and_then(|start| nearest(&packages, start)).or_else(|| packages.first());
    if let Some(start) = &start {
        beside_and_imported(root, start, &mut found);
    }
    if let Some(package) = package {
        from_package(root, package, &packages, &mut found);
    }
    if found.is_empty() && packages.len() <= 1 {
        found = searched(root);
    }
    StyleSources { sources: described(root, found), start: named.map(str::to_string), packages: Vec::new() }
}

/// Whether `path` is a folder of the Job's work, or a file that has a look, and exists.
fn is_ui(root: &Path, path: &str) -> bool {
    let ext = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    inside(root, path).is_some_and(|file| file.is_dir() || UI_EXTENSIONS.contains(&ext.as_str()))
}

/// What the file at `rel` is, or why a frame cannot use it as it stands.
pub(crate) fn usable_kind(root: &Path, rel: &str) -> Result<StyleKind, String> {
    let file = inside(root, rel).filter(|file| file.is_file()).ok_or_else(|| String::from("is not a file inside this job's checkout"))?;
    let kind = if is_style(&file) { kind_of(&file, &read_head(&file)) } else { theme_file_kind(&file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).unwrap_or(StyleKind::ThemeModule) };
    if matches!(kind, StyleKind::Scss | StyleKind::Less | StyleKind::TailwindConfig | StyleKind::ThemeModule) {
        return Err(format!("is a {} file, which a browser cannot use as it stands", kind_name(kind)));
    }
    sheet::make(root, &file).map(|_| kind).map_err(|why| format!("cannot be used: {why}"))
}

fn kind_name(kind: StyleKind) -> &'static str {
    match kind {
        StyleKind::Scss => "scss",
        StyleKind::Less => "less",
        StyleKind::TailwindConfig => "Tailwind config",
        StyleKind::ThemeModule => "theme module",
        _ => "style",
    }
}

/// `rel` as a canonical path inside `root`, or `None` where it is missing or leads out.
pub(crate) fn inside(root: &Path, rel: &str) -> Option<PathBuf> {
    let joined = root.join(rel.trim_start_matches("./"));
    let canonical = std::fs::canonicalize(joined).ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn relative(root: &Path, path: &Path) -> Option<String> {
    Some(path.strip_prefix(root).ok()?.to_string_lossy().into_owned())
}

fn skipped(name: &str) -> bool {
    name.starts_with('.') || SKIPPED.contains(&name)
}

fn is_style(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| STYLE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Every directory holding a `package.json`, down to [`DEPTH_AT_MOST`], the root's first.
fn packages_of(root: &Path) -> Vec<Package> {
    let mut found = Vec::new();
    let mut open = vec![(root.to_path_buf(), 0usize)];
    let mut visits = 0;
    while let Some((dir, depth)) = open.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut subdirs = Vec::new();
        for entry in entries.flatten() {
            visits += 1;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() && depth < DEPTH_AT_MOST && !skipped(&name) {
                subdirs.push((entry.path(), depth + 1));
            }
        }
        if visits > VISITS_AT_MOST {
            break;
        }
        if let Some(manifest) = std::fs::read_to_string(dir.join("package.json")).ok().and_then(|text| read_package_manifest(&text)) {
            found.push(Package { dir: relative(root, &dir).unwrap_or_default(), manifest });
        }
        open.extend(subdirs);
    }
    found.sort_by(|a, b| a.dir.cmp(&b.dir));
    found
}

/// The package `start` lies in: the one with the longest directory that holds it.
fn nearest<'a>(packages: &'a [Package], start: &str) -> Option<&'a Package> {
    packages
        .iter()
        .filter(|one| one.dir.is_empty() || start == one.dir || start.starts_with(&format!("{}/", one.dir)))
        .max_by_key(|one| one.dir.len())
}

fn push(found: &mut Vec<Found>, path: String, kind: StyleKind) {
    if !found.iter().any(|one| one.path == path) {
        found.push(Found { path, kind });
    }
}

/// Step 1. The stylesheets `start` imports, then those beside it.
fn beside_and_imported(root: &Path, start: &str, found: &mut Vec<Found>) {
    let Some(file) = inside(root, start) else { return };
    let dir = if file.is_dir() { file.clone() } else { file.parent().unwrap_or(root).to_path_buf() };
    if file.is_file() {
        if is_style(&file) {
            push(found, start.trim_start_matches("./").to_string(), kind_of(&file, &read_head(&file)));
        } else if let Ok(text) = std::fs::read_to_string(&file) {
            let here = relative(root, &dir).unwrap_or_default();
            for address in imported_styles(&text) {
                if let Some(path) = inside(root, &join(&here, &address)) {
                    if let Some(rel) = relative(root, &path) {
                        push(found, rel, kind_of(&path, &read_head(&path)));
                    }
                }
            }
        }
    }
    let stem = file.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default().split('.').next().unwrap_or_default().to_string();
    let mut beside: Vec<PathBuf> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_style(path) && !path.to_string_lossy().contains(".min."))
        .collect();
    beside.sort_by_key(|path| {
        let own = path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.split('.').next() == Some(stem.as_str()));
        (!own, path.clone())
    });
    for path in beside.into_iter().take(BESIDE_AT_MOST) {
        if let Some(rel) = relative(root, &path) {
            push(found, rel, kind_of(&path, &read_head(&path)));
        }
    }
}

fn join(dir: &str, address: &str) -> String {
    match dir.is_empty() {
        true => address.to_string(),
        false => format!("{dir}/{address}"),
    }
}

/// The relative addresses of style files a source file names in a string: `import "./a.css"`.
fn imported_styles(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for quote in ['"', '\''] {
        for (index, part) in text.split(quote).enumerate() {
            let lower = part.to_ascii_lowercase();
            let styled = STYLE_EXTENSIONS.iter().any(|ext| lower.ends_with(&format!(".{ext}")));
            if index % 2 == 1 && styled && part.starts_with('.') && !part.contains(char::is_whitespace) {
                found.push(part.to_string());
            }
        }
    }
    found
}

fn read_head(path: &Path) -> String {
    use std::io::Read;
    let mut text = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(64 * 1024).read_to_end(&mut text);
    }
    String::from_utf8_lossy(&text).into_owned()
}

/// What a style file is, from its name and the head of its text.
fn kind_of(path: &Path, head: &str) -> StyleKind {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_ascii_lowercase();
    match path.extension().and_then(|ext| ext.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("scss" | "sass") => StyleKind::Scss,
        Some("less") => StyleKind::Less,
        _ if name.ends_with(".module.css") => StyleKind::CssModule,
        _ if head.chars().take(1024).collect::<String>().contains("tailwindcss") => StyleKind::TailwindOutput,
        _ if has_theme_rule(head) => StyleKind::ThemeCss,
        _ => StyleKind::Css,
    }
}

fn has_theme_rule(text: &str) -> bool {
    let bare = sheet::strip_comments(text);
    let mut rest = bare.as_str();
    while let Some(at) = find_ci(rest, "@theme") {
        let after = &rest[at + 6..];
        if after.chars().next().is_some_and(|c| c.is_whitespace() || c == '{') {
            return true;
        }
        rest = after;
    }
    false
}

/// Steps 2 to 4 for `package`, and the Tailwind output or reset.
fn from_package(root: &Path, package: &Package, all: &[Package], found: &mut Vec<Found>) {
    if let Some(path) = entry_of(root, package) {
        push(found, path.clone(), kind_of(&root.join(&path), &read_head(&root.join(&path))));
    }
    let output = tailwind_output(root, &package.dir);
    if let Some(path) = &output {
        push(found, path.clone(), StyleKind::TailwindOutput);
    }
    let by_name: BTreeMap<&str, &Package> = all.iter().filter_map(|one| one.manifest.name.as_deref().map(|name| (name, one))).collect();
    let mut needed: Vec<&Package> = Vec::new();
    for name in &package.manifest.depends_on {
        if let Some(one) = by_name.get(name.as_str()).filter(|one| one.dir != package.dir) {
            needed.push(one);
        }
    }
    for one in &needed {
        if let Some(path) = entry_of(root, one) {
            push(found, path.clone(), kind_of(&root.join(&path), &read_head(&root.join(&path))));
        }
    }
    // Step 4: where the look is kept in a form a browser cannot read as it stands, or as Tailwind's theme.
    for one in std::iter::once(package).chain(needed.iter().copied()) {
        themes_of(root, one, found);
    }
    if let Some(config) = tailwind_config(root, &package.dir) {
        push(found, config, StyleKind::TailwindConfig);
    }
    let wants_reset = output.is_none() && uses_tailwind(root, package, all, found);
    if wants_reset {
        if let Some(path) = preflight(root, &package.dir) {
            push(found, path, StyleKind::TailwindReset);
        }
    }
}

/// A package's own stylesheet: its `style`, an `exports` stylesheet (preferring the one named for
/// the package), its `main` where that is CSS, or the name stylesheets are conventionally given.
fn entry_of(root: &Path, package: &Package) -> Option<String> {
    let dir = &package.dir;
    let manifest = &package.manifest;
    let tail = manifest.name.as_deref().and_then(|name| name.rsplit('/').next()).unwrap_or_default();
    let mut exported: Vec<&String> = manifest.css_exports.iter().collect();
    exported.sort_by_key(|path| {
        let stem = path.rsplit('/').next().unwrap_or_default().split('.').next().unwrap_or_default();
        stem != tail
    });
    let mut candidates: Vec<String> = Vec::new();
    candidates.extend(manifest.style.iter().cloned());
    candidates.extend(exported.into_iter().cloned());
    candidates.extend(manifest.main.iter().filter(|main| main.ends_with(".css")).cloned());
    for name in ENTRY_NAMES {
        candidates.push(format!("{name}.css"));
        candidates.push(format!("src/{name}.css"));
    }
    candidates.into_iter().map(|one| join(dir, one.trim_start_matches("./"))).find(|path| {
        let Some(file) = inside(root, path) else { return false };
        file.is_file() && kind_of(&file, &read_head(&file)) != StyleKind::ThemeCss
    })
}

/// Step 4 for one package: its Tailwind `@theme` stylesheets, SCSS and Less variables, and theme modules.
fn themes_of(root: &Path, package: &Package, found: &mut Vec<Found>) {
    for path in &package.manifest.css_exports {
        let rel = join(&package.dir, path.trim_start_matches("./"));
        if let Some(file) = inside(root, &rel).filter(|file| file.is_file()) {
            if kind_of(&file, &read_head(&file)) == StyleKind::ThemeCss {
                push(found, rel, StyleKind::ThemeCss);
            }
        }
    }
    let base = root.join(&package.dir);
    let mut visits = 0;
    let mut open = vec![(base, 0usize)];
    while let Some((dir, depth)) = open.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            visits += 1;
            let (path, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() && depth < 3 && !skipped(&name) && !is_package_root(&path, root) {
                open.push((path, depth + 1));
            } else if kind.is_file() && visits < VISITS_AT_MOST {
                let Some(rel) = relative(root, &path) else { continue };
                match theme_file_kind(&name) {
                    Some(StyleKind::Scss) => push(found, rel, StyleKind::Scss),
                    Some(StyleKind::Less) => push(found, rel, StyleKind::Less),
                    Some(StyleKind::ThemeModule) => push(found, rel, StyleKind::ThemeModule),
                    _ => {}
                }
            }
        }
    }
}

/// A folder with a manifest of its own is another package, and not part of this one's search.
fn is_package_root(dir: &Path, root: &Path) -> bool {
    dir != root && dir.join("package.json").is_file()
}

/// A file that names itself the place a project's variables or theme live.
fn theme_file_kind(name: &str) -> Option<StyleKind> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.rsplit_once('.').map_or(lower.as_str(), |(stem, _)| stem).trim_start_matches('_');
    let ext = lower.rsplit('.').next().unwrap_or_default();
    let variable_names = ["variables", "vars", "tokens", "theme", "colors", "colours"];
    match ext {
        "scss" | "sass" if variable_names.contains(&stem) => Some(StyleKind::Scss),
        "less" if variable_names.contains(&stem) => Some(StyleKind::Less),
        "ts" | "js" | "mjs" | "tsx" | "jsx" if ["theme", "tokens", "colors", "colours", "design-tokens"].contains(&stem) => {
            Some(StyleKind::ThemeModule)
        }
        _ => None,
    }
}

fn tailwind_config(root: &Path, package_dir: &str) -> Option<String> {
    let dirs = [package_dir.to_string(), String::new()];
    dirs.iter().find_map(|dir| {
        ["ts", "js", "cjs", "mjs"]
            .iter()
            .map(|ext| join(dir, &format!("tailwind.config.{ext}")))
            .find(|path| inside(root, path).is_some_and(|file| file.is_file()))
    })
}

/// A compiled Tailwind sheet a build left in `dist`, `out`, `build` or `.next`, under the cap.
fn tailwind_output(root: &Path, package_dir: &str) -> Option<String> {
    let base = root.join(package_dir);
    let mut visits = 0;
    for output in ["dist", "out", "build", ".next", "public"] {
        let mut open = vec![(base.join(output), 0usize)];
        while let Some((dir, depth)) = open.pop() {
            for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                visits += 1;
                let (path, Ok(kind)) = (entry.path(), entry.file_type()) else { continue };
                if kind.is_dir() && depth < 4 && visits < VISITS_AT_MOST {
                    open.push((path, depth + 1));
                } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "css") {
                    let small = std::fs::metadata(&path).is_ok_and(|meta| meta.len() as usize <= SHEET_AT_MOST);
                    let marked = read_head(&path).chars().take(1024).collect::<String>().contains("tailwindcss");
                    if small && marked {
                        return relative(root, &path);
                    }
                }
            }
        }
    }
    None
}

/// Whether the package, or the repository's root, builds with Tailwind, or a stylesheet found
/// imports it.
fn uses_tailwind(root: &Path, package: &Package, all: &[Package], found: &[Found]) -> bool {
    let depends = |one: &Package| one.manifest.depends_on.iter().any(|name| TAILWIND_PACKAGES.contains(&name.as_str()));
    if depends(package) || all.iter().any(|one| one.dir.is_empty() && depends(one)) {
        return true;
    }
    found.iter().any(|one| {
        let text = sheet::strip_comments(&read_head(&root.join(&one.path))).to_ascii_lowercase();
        ["@import \"tailwindcss", "@import 'tailwindcss", "@tailwind base"].iter().any(|mark| text.contains(mark))
    })
}

/// Tailwind's own reset in the installed package, found by walking `node_modules` upward.
fn preflight(root: &Path, package_dir: &str) -> Option<String> {
    let mut dir = Some(root.join(package_dir));
    while let Some(here) = dir.filter(|here| here.starts_with(root)) {
        for tail in ["node_modules/tailwindcss/preflight.css", "node_modules/tailwindcss/lib/css/preflight.css"] {
            let candidate = here.join(tail);
            if std::fs::canonicalize(&candidate).is_ok_and(|real| real.starts_with(root) && real.is_file()) {
                return relative(root, &candidate);
            }
        }
        dir = here.parent().map(Path::to_path_buf);
    }
    None
}

/// Step 5. Every plain stylesheet of the checkout, the ones that set the most colours and sizes first.
fn searched(root: &Path) -> Vec<Found> {
    let mut scored: Vec<(usize, String, StyleKind)> = Vec::new();
    let mut open = vec![root.to_path_buf()];
    let mut visits = 0;
    while let Some(dir) = open.pop() {
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            visits += 1;
            if visits > VISITS_AT_MOST {
                open.clear();
                break;
            }
            let (path, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() && !skipped(&name) {
                open.push(path);
            } else if kind.is_file() && is_style(&path) && !name.contains(".min.") {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                if text.len() > 2 * 1024 * 1024 {
                    continue;
                }
                let kind = kind_of(&path, &text);
                let score = visual_count(&tokens_in(kind, &text));
                if let (true, Some(rel)) = (score > 0, relative(root, &path)) {
                    scored.push((score, rel, kind));
                }
            }
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored.into_iter().take(SEARCHED_AT_MOST).map(|(_, path, kind)| Found { path, kind }).collect()
}

/// The answer's entries: sizes, whether a browser can use each as it stands, and tokens under a
/// budget, nearest first.
fn described(root: &Path, found: Vec<Found>) -> Vec<StyleSource> {
    let mut budget = TOKEN_BYTES_AT_MOST;
    let mut sources = Vec::new();
    let mut seen = BTreeSet::new();
    for one in found {
        if !seen.insert(one.path.clone()) {
            continue;
        }
        let Some(file) = inside(root, &one.path) else { continue };
        let bytes = std::fs::metadata(&file).map_or(0, |meta| meta.len());
        let text = if bytes <= 2 * 1024 * 1024 { std::fs::read_to_string(&file).unwrap_or_default() } else { String::new() };
        let browser_ready = !matches!(one.kind, StyleKind::Scss | StyleKind::Less | StyleKind::TailwindConfig | StyleKind::ThemeModule);
        let usable = browser_ready && sheet::make(root, &file).is_ok();
        let mut tokens: Vec<StyleToken> = Vec::new();
        for token in tokens_in(one.kind, &text) {
            let weight = token.name.len() + token.value.len() + 24;
            if weight > budget {
                continue;
            }
            budget -= weight;
            tokens.push(token);
        }
        sources.push(StyleSource { path: one.path, bytes, kind: one.kind, usable, tokens });
    }
    sources
}

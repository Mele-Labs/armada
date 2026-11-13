//! A repository's components: each file named for one, the classes its own stylesheet defines, and
//! a markup sample read from the tags in the file that carry those classes. **Nothing runs**: the
//! sample is the elements as written, so a class chosen at run time shows only where it is a
//! literal somewhere in the file.

use std::path::Path;

use ipc::CatalogueComponent;

use super::super::sheet::strip_comments;

/// Most bytes of a component file read.
const READ_AT_MOST: u64 = 200 * 1024;
/// Most classes listed for one component.
const CLASSES_AT_MOST: usize = 40;
/// Most characters of one sample.
const SAMPLE_AT_MOST: usize = 1_500;
/// The characters a `<` follows where it opens a tag, and not a comparison or a type's parameters.
const BEFORE_A_TAG: [char; 13] = ['(', '>', '}', '{', '?', ':', ',', '=', '&', '|', '[', ';', '\n'];

pub(crate) fn read(root: &Path, files: &[String], sheets: &[String]) -> Vec<CatalogueComponent> {
    let mut found = Vec::new();
    for file in files {
        let path = root.join(file);
        if std::fs::metadata(&path).map_or(true, |meta| meta.len() > READ_AT_MOST) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let name = file.rsplit('/').next().unwrap_or_default().split('.').next().unwrap_or_default().to_string();
        let sheet = own_sheet(file, &name, sheets);
        let sheet_classes = sheet
            .as_ref()
            .and_then(|sheet| std::fs::read_to_string(root.join(sheet)).ok())
            .map(|css| classes_in_sheet(&css))
            .unwrap_or_default();
        let elements = elements_in(&text);
        let keep = |class: &str| sheet_classes.is_empty() || sheet_classes.iter().any(|one| one == class);
        let tree = kept(elements, &keep);
        let mut classes = sheet_classes.clone();
        if classes.is_empty() {
            collect_classes(&tree, &mut classes);
        }
        if classes.is_empty() {
            continue;
        }
        classes.truncate(CLASSES_AT_MOST);
        let sample = rendered(&tree);
        found.push(CatalogueComponent { name, file: file.clone(), sheet, classes, sample: (!sample.is_empty()).then_some(sample) });
    }
    found
}

/// The stylesheet beside `file` named for the component: `Kbd.css`, `Kbd.module.css`, `Kbd.scss`.
fn own_sheet(file: &str, name: &str, sheets: &[String]) -> Option<String> {
    let dir = file.rsplit_once('/').map(|(dir, _)| dir).unwrap_or_default();
    sheets
        .iter()
        .find(|sheet| {
            let (here, sheet_name) = sheet.rsplit_once('/').unwrap_or(("", sheet.as_str()));
            here == dir && sheet_name.split('.').next() == Some(name)
        })
        .cloned()
}

/// Every class a stylesheet's selectors name, first to last, each once.
pub(crate) fn classes_in_sheet(css: &str) -> Vec<String> {
    let bare = strip_comments(css);
    let mut found: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut in_selector = true;
    let chars: Vec<(usize, char)> = bare.char_indices().collect();
    for (index, &(at, ch)) in chars.iter().enumerate() {
        match ch {
            '{' => {
                depth += 1;
                in_selector = true;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                in_selector = true;
            }
            ':' if depth > 0 => {
                // A declaration's value follows until `;` or `}`; a nested selector's pseudo-class does not.
                let rest = &bare[at..];
                let line_end = rest.find([';', '{', '}']).unwrap_or(rest.len());
                if !rest[..line_end].contains('{') && rest.as_bytes().get(line_end) != Some(&b'{') {
                    in_selector = false;
                }
            }
            ';' => in_selector = true,
            '.' if in_selector => {
                let before = index.checked_sub(1).map(|i| chars[i].1);
                if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                    continue;
                }
                let name: String = bare[at + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
                let starts_well = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '-');
                if starts_well && !found.contains(&name) {
                    found.push(name);
                }
            }
            _ => {}
        }
    }
    found
}

/// An element of a component's markup, with the classes its `className` or `class` names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Element {
    pub tag: String,
    pub classes: Vec<String>,
    pub children: Vec<Element>,
}

/// Every tag of `text` as a tree, components and fragments included, as written.
pub(crate) fn elements_in(text: &str) -> Vec<Element> {
    let mut roots: Vec<Element> = Vec::new();
    let mut open: Vec<Element> = Vec::new();
    let bytes = text.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] != b'<' {
            at += 1;
            continue;
        }
        if bytes.get(at + 1) == Some(&b'/') {
            let end = text[at..].find('>').map_or(text.len(), |end| at + end);
            let name = text[at + 2..end].trim();
            if let Some(depth) = open.iter().rposition(|one| one.tag == name) {
                while open.len() > depth {
                    let Some(done) = open.pop() else { break };
                    attach(&mut open, &mut roots, done);
                }
            }
            at = end + 1;
            continue;
        }
        if bytes.get(at + 1) == Some(&b'>') {
            open.push(Element { tag: String::new(), classes: Vec::new(), children: Vec::new() });
            at += 2;
            continue;
        }
        let starts = bytes.get(at + 1).is_some_and(|byte| byte.is_ascii_alphabetic());
        if !starts || !opens_a_tag(text, at, !open.is_empty()) {
            at += 1;
            continue;
        }
        let name: String = text[at + 1..].chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '_')).collect();
        let Some((attributes, end)) = attributes_of(text, at + 1 + name.len()) else { break };
        let element = Element { tag: name, classes: classes_of(attributes), children: Vec::new() };
        if attributes.trim_end().ends_with('/') {
            attach(&mut open, &mut roots, element);
        } else {
            open.push(element);
        }
        at = end + 1;
    }
    while let Some(done) = open.pop() {
        attach(&mut open, &mut roots, done);
    }
    roots
}

fn attach(open: &mut [Element], roots: &mut Vec<Element>, done: Element) {
    match open.last_mut() {
        Some(parent) => parent.children.push(done),
        None => roots.push(done),
    }
}

/// Whether the `<` at `at` opens a tag: what comes before it is where markup can stand.
fn opens_a_tag(text: &str, at: usize, inside_markup: bool) -> bool {
    let before = text[..at].trim_end_matches([' ', '\t']);
    let Some(last) = before.chars().next_back() else { return true };
    if BEFORE_A_TAG.contains(&last) || before.ends_with("return") {
        return true;
    }
    // Between tags, text may stand before one: `Press <kbd>`. Only an identifier right before it
    // is a type's parameters.
    inside_markup && before.len() < at
}

/// The attributes of the tag whose name ends at `from`, and where its `>` is.
fn attributes_of(text: &str, from: usize) -> Option<(&str, usize)> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for (offset, ch) in text[from..].char_indices() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(ch),
            (None, '{') => depth += 1,
            (None, '}') => depth = depth.saturating_sub(1),
            (None, '>') if depth == 0 => return Some((&text[from..from + offset], from + offset)),
            _ => {}
        }
    }
    None
}

/// The class names a tag's `className` or `class` holds as literals.
fn classes_of(attributes: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for key in ["className=", "class="] {
        let mut from = 0;
        while let Some(at) = attributes[from..].find(key).map(|at| from + at) {
            from = at + key.len();
            let before = attributes[..at].chars().next_back();
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-') {
                continue;
            }
            let value = value_at(&attributes[from..]);
            for literal in literals(value) {
                for word in literal.split_whitespace() {
                    let usable = word.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '-')
                        && word.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '/' | '[' | ']' | '.' | '%' | '#' | '(' | ')' | ','));
                    if usable && !found.iter().any(|one| one == word) {
                        found.push(word.to_string());
                    }
                }
            }
        }
    }
    found
}

/// The attribute value that starts `text`: a quoted string, or a `{…}` expression.
fn value_at(text: &str) -> &str {
    let Some(first) = text.chars().next() else { return "" };
    let close = match first {
        '"' | '\'' => text[1..].find(first).map(|end| end + 2),
        '{' => {
            let mut depth = 0usize;
            let mut end = None;
            for (at, ch) in text.char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(at + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            end
        }
        _ => None,
    };
    &text[..close.unwrap_or(text.len()).min(text.len())]
}

/// The string literals in an attribute value, a template's `${…}` parts left out.
fn literals(value: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut quote: Option<char> = None;
    let mut current = String::new();
    let mut hole = 0usize;
    let chars: Vec<char> = value.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        match quote {
            None if matches!(ch, '"' | '\'' | '`') => quote = Some(ch),
            None => {}
            Some('`') if hole == 0 && ch == '$' && chars.get(index + 1) == Some(&'{') => {
                hole = 1;
                current.push(' ');
                index += 1;
            }
            Some('`') if hole > 0 => match ch {
                '{' => hole += 1,
                '}' => hole -= 1,
                _ => {}
            },
            Some(q) if ch == q => {
                found.push(std::mem::take(&mut current));
                quote = None;
            }
            Some(_) => current.push(ch),
        }
        index += 1;
    }
    found
}

/// `elements` with only the tags that carry a class `keep` accepts, each attached to the nearest
/// such tag above it.
fn kept(elements: Vec<Element>, keep: &dyn Fn(&str) -> bool) -> Vec<Element> {
    let mut out = Vec::new();
    for element in elements {
        let html = element.tag.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && !element.tag.contains('.');
        let classes: Vec<String> = element.classes.iter().filter(|class| keep(class)).cloned().collect();
        let children = kept(element.children, keep);
        if html && !classes.is_empty() {
            out.push(Element { tag: element.tag, classes, children });
        } else {
            out.extend(children);
        }
    }
    out
}

fn collect_classes(tree: &[Element], into: &mut Vec<String>) {
    for element in tree {
        for class in &element.classes {
            if !into.contains(class) {
                into.push(class.clone());
            }
        }
        collect_classes(&element.children, into);
    }
}

/// The tree as indented HTML, roots dropped once past [`SAMPLE_AT_MOST`].
fn rendered(tree: &[Element]) -> String {
    let mut out = String::new();
    for root in tree {
        let mut one = String::new();
        render(root, 0, &mut one);
        if !out.is_empty() && out.len() + one.len() > SAMPLE_AT_MOST {
            break;
        }
        out.push_str(&one);
    }
    out.trim_end().to_string()
}

fn render(element: &Element, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let classes = element.classes.join(" ");
    if element.children.is_empty() {
        out.push_str(&format!("{indent}<{tag} class=\"{classes}\"></{tag}>\n", tag = element.tag));
        return;
    }
    out.push_str(&format!("{indent}<{} class=\"{classes}\">\n", element.tag));
    for child in &element.children {
        render(child, depth + 1, out);
    }
    out.push_str(&format!("{indent}</{}>\n", element.tag));
}

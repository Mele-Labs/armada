//! What a page answered about the part of it at a point, bounded and worded. **The bound belongs on the side that is not the
//! page**: Classic's `capture/bounds.ts` (`rectOf`, `aimed`, `bounded`, `readOf`), re-applied here on the answer of Classic's own
//! layer (`capture/layer.ts`, built into `read.js`), since what comes back is the page's own words about itself. Pure.

use serde_json::Value;

/// The most a name or an owner may run to, as `bounds.ts` has it.
const MOST_NAME: usize = 120;
/// The most an element's text, a nearby name or a marked state keeps: the layer's own figure.
pub const MOST_TEXT: usize = 80;
const MOST_SELECTOR: usize = 600;
const MOST_LOCATION: usize = 600;
const MOST_OWNERS: usize = 16;
const MOST_TAG: usize = 40;
const MOST_NEARBY: usize = 8;
const MOST_SELECTED: usize = 12;
/// The most a whole answer may be before it is not read at all: the layer's markup is cut at 2000 characters, so this is far past
/// any answer it gives.
pub const MOST_ANSWER: usize = 64 * 1024;

/// A box in the page's viewport, in CSS pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What the pointer is over, as an aim answers it: the part's box and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Aimed {
    pub rect: Rect,
    pub words: Words,
}

/// What a part is called: its role and accessible name where it has them, and its tag.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Words {
    pub role: String,
    pub name: String,
    pub tag: String,
}

/// A part of the page held for a note: `bounds.ts`'s `bounded` and `readOf` together, less the markup and styles a note does not carry.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub rect: Rect,
    pub words: Words,
    pub text: String,
    pub label: Option<String>,
    pub selector: String,
    pub component: Option<String>,
    pub owners: Vec<String>,
    pub screen: Option<String>,
    pub layer: Option<String>,
    pub location: String,
    pub nearby: Vec<String>,
    pub stamp: Option<String>,
    pub selected: Vec<String>,
}

fn text(value: &Value, most: usize) -> Option<String> {
    value.as_str().map(|text| text.chars().take(most).collect())
}

/// A number rounded to a whole one, or 0 for anything that is not a finite number.
fn whole(value: &Value) -> f64 {
    value.as_f64().filter(|number| number.is_finite()).map_or(0., f64::round)
}

/// A rectangle, clamped so a negative or an infinite one cannot reach a ring.
pub fn rect_of(value: &Value, viewport: (f64, f64)) -> Rect {
    let x = whole(&value["x"]).clamp(0., viewport.0.max(0.));
    let y = whole(&value["y"]).clamp(0., viewport.1.max(0.));
    Rect {
        x,
        y,
        width: whole(&value["width"]).clamp(0., (viewport.0 - x).max(0.)),
        height: whole(&value["height"]).clamp(0., (viewport.1 - y).max(0.)),
    }
}

fn words_of(answer: &Value) -> Words {
    Words {
        role: text(&answer["role"], MOST_NAME).unwrap_or_default().trim().to_string(),
        name: text(&answer["name"], MOST_NAME).unwrap_or_default().trim().to_string(),
        tag: text(&answer["named"], MOST_TAG).unwrap_or_default(),
    }
}

/// The box and the words an aim answers with, or none where it answered nothing.
pub fn aimed(answer: &Value, viewport: (f64, f64)) -> Option<Aimed> {
    let said = answer.as_object()?;
    let rect = rect_of(said.get("box")?, viewport);
    (rect.width > 0. && rect.height > 0.).then(|| Aimed { rect, words: words_of(answer) })
}

fn strings(value: &Value, most: usize, longest: usize) -> Vec<String> {
    value.as_array().map(|all| all.iter().filter_map(|one| text(one, longest)).filter(|one| !one.is_empty()).take(most).collect()).unwrap_or_default()
}

/// Whether a stamp is shaped `path:line`: a path with no space, a colon, a line from 1. Anything else is dropped.
fn stamp_shaped(stamp: &str) -> bool {
    let Some((path, line)) = stamp.rsplit_once(':') else { return false };
    let mut path_chars = path.chars();
    path_chars.next().is_some_and(|first| first != ':' && !first.is_whitespace())
        && !path.chars().any(char::is_whitespace)
        && line.chars().next().is_some_and(|first| ('1'..='9').contains(&first))
        && line.chars().all(|digit| digit.is_ascii_digit())
}

/// The part a take answered with, every field re-typed and re-bounded, or none where it is not one.
pub fn held(answer: &Value, viewport: (f64, f64)) -> Option<Read> {
    let said = answer.as_object()?;
    let capture = said.get("capture").filter(|capture| capture.is_object())?;
    let element = &capture["element"];
    let tag = text(&element["tag"], MOST_TAG)?;
    let selector = text(&capture["selector"], MOST_SELECTOR)?;
    // The layer always sends its markup, so an answer without it is not the layer's.
    capture["markup"].as_str()?;
    let mut words = words_of(answer);
    words.tag = tag;
    let owners = capture["owners"].as_array().map(|all| all.iter().take(MOST_OWNERS).filter_map(|one| text(one, MOST_NAME)).collect()).unwrap_or_default();
    let state = &capture["state"];
    Some(Read {
        rect: rect_of(&capture["bounds"], viewport),
        words,
        text: text(&element["text"], MOST_TEXT).unwrap_or_default(),
        label: text(&element["label"], MOST_TEXT),
        selector,
        component: text(&capture["component"], MOST_NAME),
        owners,
        screen: text(&capture["screen"], MOST_NAME),
        layer: text(&capture["layer"], MOST_NAME),
        location: text(&capture["location"], MOST_LOCATION).unwrap_or_else(|| "/".to_string()),
        nearby: strings(&capture["nearby"], MOST_NEARBY, MOST_TEXT),
        stamp: text(&capture["stamp"], MOST_LOCATION).filter(|stamp| stamp_shaped(stamp)),
        selected: strings(&state["selected"], MOST_SELECTED, MOST_NAME),
    })
}

/// A page's host, with its port where it has one: `fast.com` for `https://fast.com/start`. None for an address that is not a web
/// page's, such as `about:blank` or a file.
pub fn host_of(address: &str) -> Option<String> {
    let rest = address.strip_prefix("https://").or_else(|| address.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?;
    (!host.is_empty()).then(|| host.to_string())
}

/// What a part of a page is called on the note card: the page's host, then the part's role word and its accessible name, as
/// `fast.com › button · Google Search`. A part with no role is said by its tag, and one with no name by its own text, and none by
/// neither of them alone.
pub fn said(host: Option<&str>, words: &Words, text: &str) -> String {
    let role = if words.role.is_empty() { words.tag.as_str() } else { words.role.as_str() };
    let name = if words.name.is_empty() { text.trim() } else { words.name.as_str() };
    let part = match (role.is_empty(), name.is_empty()) {
        (_, true) => role.to_string(),
        (true, false) => name.to_string(),
        (false, false) => format!("{role} · {name}"),
    };
    match host {
        Some(host) if part.is_empty() => host.to_string(),
        Some(host) => format!("{host} › {part}"),
        None => part,
    }
}

/// The `scenario` a page's location names in its query, as `sessionNoteBody` reads it.
pub fn scenario_of(location: &str) -> Option<String> {
    let query = location.split('#').next()?.split_once('?')?.1;
    query.split('&').find_map(|pair| pair.strip_prefix("scenario=")).filter(|scenario| !scenario.is_empty()).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const VIEWPORT: (f64, f64) = (1200., 800.);

    fn take(capture: Value) -> Value {
        json!({ "box": { "x": 1, "y": 2, "width": 3, "height": 4 }, "named": "button", "role": "button", "name": "Google Search", "capture": capture })
    }

    fn capture() -> Value {
        json!({
            "selector": "form > input:nth-of-type(1)",
            "element": { "tag": "input", "text": "", "label": "Search" },
            "location": "/search?scenario=wide#top",
            "bounds": { "x": 10, "y": 20, "width": 150, "height": 30 },
            "window": { "width": 1200, "height": 800 },
            "markup": "<input>",
            "nearby": ["Main", "Search"],
            "state": { "selected": ["tab \"Web\""] },
            "stamp": "src/Search.tsx:42",
        })
    }

    #[test]
    fn a_part_is_said_by_its_host_then_its_role_word_and_accessible_name() {
        let words = |role: &str, name: &str, tag: &str| Words { role: role.into(), name: name.into(), tag: tag.into() };
        assert_eq!(said(Some("fast.com"), &words("button", "Google Search", "input"), ""), "fast.com › button · Google Search");
        assert_eq!(said(None, &words("button", "Save", "button"), ""), "button · Save");
        assert_eq!(said(Some("fast.com"), &words("", "", "div"), ""), "fast.com › div", "no role and no name: its tag");
        assert_eq!(said(Some("fast.com"), &words("", "", "div"), "  Plain words "), "fast.com › div · Plain words", "no name: its own text");
        assert_eq!(said(Some("localhost:5173"), &words("main", "", "main"), ""), "localhost:5173 › main");
        assert_eq!(said(Some("fast.com"), &words("", "", ""), ""), "fast.com");
    }

    #[test]
    fn a_host_is_the_authority_of_a_web_address_and_nothing_for_any_other() {
        assert_eq!(host_of("https://fast.com/start?x=1").as_deref(), Some("fast.com"));
        assert_eq!(host_of("http://localhost:5173/#a").as_deref(), Some("localhost:5173"));
        assert_eq!(host_of("https://me:secret@example.org/a").as_deref(), Some("example.org"), "a sign-in is never said");
        assert_eq!(host_of("about:blank"), None);
        assert_eq!(host_of("file:///tmp/a.html"), None);
        assert_eq!(host_of("https:///a"), None);
    }

    #[test]
    fn a_box_is_clamped_to_the_viewport_whatever_the_page_says() {
        let rect = rect_of(&json!({ "x": -50, "y": 790.4, "width": 99999, "height": 99999 }), VIEWPORT);
        assert_eq!(rect, Rect { x: 0., y: 790., width: 1200., height: 10. });
        assert_eq!(rect_of(&json!({ "x": "far", "y": null, "width": f64::INFINITY, "height": [] }), VIEWPORT), Rect { x: 0., y: 0., width: 0., height: 0. }, "not numbers: nothing");
        assert_eq!(rect_of(&json!({ "x": 2000, "y": 5, "width": 10, "height": 10 }), VIEWPORT).width, 0., "a box past the edge has no width left");
    }

    #[test]
    fn an_aim_with_no_box_or_an_empty_one_answers_nothing_and_its_words_are_cut() {
        assert_eq!(aimed(&Value::Null, VIEWPORT), None);
        assert_eq!(aimed(&json!({ "named": "div" }), VIEWPORT), None);
        assert_eq!(aimed(&json!({ "box": { "x": 1, "y": 1, "width": 0, "height": 5 } }), VIEWPORT), None);
        let long = "n".repeat(500);
        let aim = aimed(&json!({ "box": { "x": 1, "y": 1, "width": 5, "height": 5 }, "role": "button", "name": long, "named": "button" }), VIEWPORT).unwrap();
        assert_eq!(aim.words.name.chars().count(), MOST_NAME);
        assert_eq!(aim.words.role, "button");
    }

    #[test]
    fn a_take_is_every_field_retyped_and_rebounded_and_the_page_cannot_add_one() {
        let read = held(&take(capture()), VIEWPORT).unwrap();
        assert_eq!(read.rect, Rect { x: 10., y: 20., width: 150., height: 30. });
        assert_eq!((read.words.role.as_str(), read.words.name.as_str(), read.words.tag.as_str()), ("button", "Google Search", "input"));
        assert_eq!(read.label.as_deref(), Some("Search"));
        assert_eq!(read.location, "/search?scenario=wide#top");
        assert_eq!(read.nearby, ["Main", "Search"]);
        assert_eq!(read.selected, ["tab \"Web\""]);
        assert_eq!(read.stamp.as_deref(), Some("src/Search.tsx:42"));

        let mut hostile = capture();
        hostile["selector"] = json!("s".repeat(5000));
        hostile["owners"] = json!((0..40).map(|one| format!("Owner{one}")).collect::<Vec<_>>());
        hostile["nearby"] = json!((0..40).map(|one| format!("n{one}")).collect::<Vec<_>>());
        hostile["state"] = json!({ "selected": (0..40).map(|one| format!("s{one}")).collect::<Vec<_>>() });
        hostile["element"] = json!({ "tag": "t".repeat(90), "text": "x".repeat(500) });
        hostile["bounds"] = json!({ "x": -5, "y": -5, "width": 1e9, "height": 1e9 });
        let read = held(&take(hostile), VIEWPORT).unwrap();
        assert_eq!(read.selector.chars().count(), 600);
        assert_eq!(read.owners.len(), 16);
        assert_eq!(read.nearby.len(), 8);
        assert_eq!(read.selected.len(), 12);
        assert_eq!(read.words.tag.chars().count(), 40);
        assert_eq!(read.text.chars().count(), 80);
        assert_eq!(read.rect, Rect { x: 0., y: 0., width: 1200., height: 800. });
    }

    #[test]
    fn an_answer_that_is_not_the_layers_is_not_a_take() {
        assert_eq!(held(&Value::Null, VIEWPORT), None);
        assert_eq!(held(&json!({ "box": {} }), VIEWPORT), None, "no capture");
        let mut no_markup = capture();
        no_markup.as_object_mut().unwrap().remove("markup");
        assert_eq!(held(&take(no_markup), VIEWPORT), None);
        let mut no_selector = capture();
        no_selector["selector"] = json!(7);
        assert_eq!(held(&take(no_selector), VIEWPORT), None);
    }

    #[test]
    fn a_stamp_not_shaped_path_and_line_is_dropped() {
        for stamp in ["src/a.tsx:42", "a:1", "C:\\a.tsx:9"] {
            assert!(stamp_shaped(stamp), "{stamp}");
        }
        for stamp in ["src/a.tsx", "src/a.tsx:0", "src/a.tsx:x", ":12", "a b.tsx:3", "a.tsx:03", "a.tsx:"] {
            assert!(!stamp_shaped(stamp), "{stamp}");
        }
    }

    #[test]
    fn the_scenario_is_read_off_the_pages_own_query() {
        assert_eq!(scenario_of("/a?scenario=wide#x").as_deref(), Some("wide"));
        assert_eq!(scenario_of("/a?x=1&scenario=narrow").as_deref(), Some("narrow"));
        assert_eq!(scenario_of("/a?scenario="), None);
        assert_eq!(scenario_of("/a#scenario=wide"), None, "a hash is not a query");
        assert_eq!(scenario_of("/a"), None);
    }
}

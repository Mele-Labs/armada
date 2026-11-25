//! What Bridge runs inside a Job's page, as the bodies of async functions (`callAsyncJavaScript`) in two content worlds of
//! Bridge's own, which share the page's DOM and nothing else: the finder's, and the agent's expressions'.
//!
//! **The finder is Classic's own** (`page.js`, built from `walk-find-page.ts`), so an agent reads one snapshot and names one kind of
//! target in either app. It holds the element it found, so it is installed once a page load (`typeof ArmadaFinder`), and
//! whatever a script answers is a short string that Bridge reads.

use ipc::WalkTarget;

use super::acts::{evaluate_source, MOST_SNAPSHOT};

/// The world the finder, and what presses and types, run in.
pub const FINDER_WORLD: &str = "armada-job-browser-finder";
/// The world an agent's expression runs in, apart from the finder's so it cannot reach `ArmadaFinder`.
pub const EVALUATE_WORLD: &str = "armada-job-browser-evaluate";

/// Classic's finder, as built (`classic-final:apps/desktop/scripts/job-browser-page.mjs`). Never edited here.
const FINDER: &str = include_str!("page.js");

/// A page nobody is looking at is in no window, or hidden, and WebKit does not draw it, so it never calls an animation frame back and
/// the finder's wait for a target to hold still would never end. In Bridge's own worlds a frame is a timer.
const FRAMES: &str = "window.requestAnimationFrame = (call) => setTimeout(() => call(performance.now()), 16);";

/// The script that installs the finder in the finder's world where it is not there yet: a page load clears the world, and a second
/// copy would forget the element the first holds.
pub fn install() -> String {
    format!("if (typeof ArmadaFinder === \"undefined\") {{\n{FRAMES}\n{FINDER}\n}}\n\"installed\"")
}

fn encoded(target: &WalkTarget) -> String {
    ipc::encode(target).unwrap_or_else(|_| "null".into())
}

/// Look for the target, scroll it into view and wait for it to hold still; hold it for `press` and `fill`. Answers `found` or `none`.
pub fn arrive(target: &WalkTarget, patience_ms: u64) -> String {
    format!("const seen = await ArmadaFinder.finder.arrive({}, {patience_ms});\nreturn seen === null ? \"none\" : \"found\";", encoded(target))
}

/// Press the held element as a person's pointer does, at the centre of its box: the pointer and mouse events, then `click()`.
/// **Dispatched in the page, not sent as input**: WebKit takes no synthetic `NSEvent` for a page that is in no window, where Classic
/// sends real input to a view it keeps laid out. A page hears the same events either way, apart from their `isTrusted`. Answers `done`,
/// or `gone` where the element left the page.
pub fn press() -> String {
    "const box = ArmadaFinder.finder.box();
if (box === null) return \"gone\";
const x = box.x + box.width / 2;
const y = box.y + box.height / 2;
const hit = document.elementFromPoint(x, y);
if (hit === null) return \"gone\";
const mouse = { bubbles: true, cancelable: true, composed: true, view: window, clientX: x, clientY: y, button: 0, buttons: 1 };
const pointer = { ...mouse, pointerId: 1, pointerType: \"mouse\", isPrimary: true };
hit.dispatchEvent(new PointerEvent(\"pointerdown\", pointer));
hit.dispatchEvent(new MouseEvent(\"mousedown\", mouse));
const takes = hit.closest(\"input, textarea, select, button, a[href], [tabindex], [contenteditable]\");
if (takes !== null && typeof takes.focus === \"function\") takes.focus({ preventScroll: true });
hit.dispatchEvent(new PointerEvent(\"pointerup\", { ...pointer, buttons: 0 }));
hit.dispatchEvent(new MouseEvent(\"mouseup\", { ...mouse, buttons: 0 }));
hit.click();
return \"done\";"
        .to_string()
}

/// Give the held element the keyboard; for a field, select what is in it and type `text` over it (`insertText`, which the page
/// hears as typing, else the field's own value setter and an input and a change event); then Enter where `submit`: the key events,
/// and the form's `requestSubmit()` where the keydown was not prevented. Answers `done`, or `gone` where the element left the page.
pub fn fill(text: &str, submit: bool) -> String {
    let words = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    format!(
        "const kind = ArmadaFinder.finder.focus();
if (kind === null) return \"gone\";
const text = {words};
const field = document.activeElement;
if (kind === \"field\") {{
  try {{
    if (field !== null && typeof field.select === \"function\") field.select();
    else document.execCommand(\"selectAll\");
  }} catch (error) {{}}
  let typed = false;
  try {{
    typed = document.execCommand(\"insertText\", false, text);
  }} catch (error) {{}}
  if (!typed && field !== null) {{
    const proto = field instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    const set = Object.getOwnPropertyDescriptor(proto, \"value\");
    if (set && set.set && \"value\" in field) set.set.call(field, text);
    else if (field.isContentEditable) field.textContent = text;
    field.dispatchEvent(new Event(\"input\", {{ bubbles: true }}));
    field.dispatchEvent(new Event(\"change\", {{ bubbles: true }}));
  }}
}}
if ({submit}) {{
  const at = document.activeElement || document.body;
  const key = {{ key: \"Enter\", code: \"Enter\", keyCode: 13, which: 13, bubbles: true, cancelable: true, composed: true }};
  const allowed = at.dispatchEvent(new KeyboardEvent(\"keydown\", key));
  at.dispatchEvent(new KeyboardEvent(\"keypress\", key));
  at.dispatchEvent(new KeyboardEvent(\"keyup\", key));
  if (allowed) {{
    if (at instanceof HTMLTextAreaElement) document.execCommand(\"insertText\", false, \"\\n\");
    else {{
      const form = at.closest ? at.closest(\"form\") : null;
      if (form !== null) {{
        if (typeof form.requestSubmit === \"function\") form.requestSubmit();
        else form.submit();
      }}
    }}
  }}
}}
return \"done\";"
    )
}

/// The page's accessibility tree as text, cut at the most a snapshot may be.
pub fn snapshot() -> String {
    format!("return ArmadaFinder.finder.snapshot({MOST_SNAPSHOT});")
}

/// The agent's expression, in a world of its own: awaited, its value as `JSON.stringify` writes it. Answers `r` and that text,
/// `n` where the value has none, or `t` and the message of what it threw.
pub fn evaluate(expression: &str) -> String {
    format!(
        "{FRAMES}
try {{
  const said = await {};
  return said === undefined ? \"n\" : \"r\" + said;
}} catch (error) {{
  return \"t\" + String(error && error.message ? error.message : error);
}}",
        evaluate_source(expression)
    )
}

/// What `evaluate` answered, read: the value's JSON, none where it had none, or the throw.
pub fn evaluated(answer: Option<&str>) -> Result<Option<String>, String> {
    match answer {
        Some(said) if said.starts_with('r') => Ok(Some(said[1..].to_string())),
        Some("n") => Ok(None),
        Some(said) if said.starts_with('t') => Err(said[1..].to_string()),
        _ => Err("The page gave no answer.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(role: &str, name: &str) -> WalkTarget {
        WalkTarget { role: role.into(), name: Some(name.into()), exact: None, within: None }
    }

    #[test]
    fn the_finder_is_installed_once_a_page_load_and_with_frames_that_are_timers() {
        let script = install();
        assert!(script.starts_with("if (typeof ArmadaFinder === \"undefined\") {"));
        assert!(script.contains("window.requestAnimationFrame = (call) => setTimeout("));
        assert!(script.contains("var ArmadaFinder="), "Classic's finder, as built");
        assert!(script.ends_with("\"installed\""));
    }

    #[test]
    fn a_target_is_sent_as_the_wire_spells_it_with_the_time_to_wait() {
        let script = arrive(&target("button", "Save"), 5000);
        assert!(script.contains(r#"ArmadaFinder.finder.arrive({"role":"button","name":"Save"}, 5000)"#), "{script}");
    }

    #[test]
    fn a_press_is_pointer_and_mouse_events_then_a_click_at_the_centre_of_the_box() {
        let script = press();
        let order: Vec<usize> = ["pointerdown", "mousedown", "pointerup", "mouseup", "hit.click()"].iter().map(|word| script.find(word).expect(word)).collect();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
        assert!(script.contains("box.x + box.width / 2"));
    }

    #[test]
    fn typing_carries_the_text_as_a_json_string_that_cannot_end_the_script() {
        let script = fill("a\"b\nc</script>\u{2028}", false);
        assert!(script.contains(r#"const text = "a\"b\nc</script>"#), "{script}");
        assert!(script.contains("if (false) {"));
        assert!(fill("x", true).contains("if (true) {"));
        assert!(script.contains("document.execCommand(\"insertText\", false, text)"));
        assert!(script.contains("requestSubmit"));
    }

    #[test]
    fn an_expression_is_run_in_its_own_world_and_answered_as_a_value_nothing_or_a_throw() {
        assert_ne!(FINDER_WORLD, EVALUATE_WORLD);
        assert!(evaluate("1 + 1").contains("const value = await (\n1 + 1\n);"));
        assert_eq!(evaluated(Some("r42")), Ok(Some("42".into())));
        assert_eq!(evaluated(Some("r\"a\"")), Ok(Some("\"a\"".into())));
        assert_eq!(evaluated(Some("n")), Ok(None));
        assert_eq!(evaluated(Some("tboom is not defined")), Err("boom is not defined".into()));
        assert_eq!(evaluated(None), Err("The page gave no answer.".into()));
    }
}

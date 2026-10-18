import { afterEach, expect, test } from "vitest";

import { SOURCE_ATTRIBUTE } from "../../../shared/annotations";
import { capture } from "./capture";

afterEach(() => {
  document.body.innerHTML = "";
});

/** A rail item as `Sidebar` draws one: glyph, label, count, and the binding revealed under Cmd. */
function rail(): Element {
  document.body.innerHTML = `
    <nav>
      <button aria-current="page" class="armada-sidebar__item">
        <svg aria-hidden="true"></svg>
        <span class="armada-sidebar__label">Job Board</span>
        <span class="armada-sidebar__count">22</span>
        <span class="armada-sidebar__kbd"><kbd>⌘</kbd><kbd>2</kbd></span>
      </button>
    </nav>
    <main><p id="target">No jobs. Propose one above.</p></main>`;
  return document.getElementById("target")!;
}

test("a note names the screen by the selected tab, the strip's page before the title bar's", () => {
  document.body.innerHTML = `
    <nav aria-label="Places"><button role="tab" aria-selected="false">Cockpit</button><button role="tab" aria-selected="true">System</button></nav>
    <nav aria-label="System"><button role="tab" aria-selected="false">Kit</button><button role="tab" aria-selected="true">Doctor</button></nav>
    <main><p id="target">All green.</p></main>`;
  expect(capture(document.getElementById("target")!, new Date("2026-10-14T14:00:00Z")).screen).toBe("Doctor");
});

test("a note names the screen by the rail item's label, not its count or binding", () => {
  expect(capture(rail(), new Date("2026-09-17T14:00:00Z")).screen).toBe("Job Board");
});

/** A row as the build stamps one: the JSX that drew each element, on each element. */
function stamped(): Element {
  document.body.innerHTML = `
    <main ${SOURCE_ATTRIBUTE}="packages/screens/src/Board.tsx:31">
      <div ${SOURCE_ATTRIBUTE}="packages/screens/src/JobRow.tsx:88">
        <span id="named">42-add-an-illustration</span>
        <b id="deep" ${SOURCE_ATTRIBUTE}="packages/screens/src/JobRow.tsx:94">Waiting on you</b>
      </div>
    </main>`;
  return document.getElementById("named")!;
}

const AT = new Date("2026-09-22T14:00:00Z");

test("a note carries the JSX that drew the element it was pinned on", () => {
  stamped();
  expect(capture(document.getElementById("deep")!, AT).source).toEqual({
    file: "packages/screens/src/JobRow.tsx",
    line: 94,
  });
});

test("an element the build did not stamp takes the nearest stamped ancestor, not the outermost", () => {
  expect(capture(stamped(), AT).source).toEqual({ file: "packages/screens/src/JobRow.tsx", line: 88 });
});

test("a note on an element with nothing stamped above it carries no source at all", () => {
  document.body.innerHTML = `<main><p id="target">No jobs. Propose one above.</p></main>`;
  const note = capture(document.getElementById("target")!, AT);
  expect(note.source).toBeUndefined();
  expect("source" in note).toBe(false);
});

test("a stamp that is not a file and a line is read as none rather than guessed at", () => {
  for (const value of ["packages/screens/src/Board.tsx", "packages/screens/src/Board.tsx:none", ":12"]) {
    document.body.innerHTML = `<div ${SOURCE_ATTRIBUTE}="${value}"><p id="target">Waiting on you</p></div>`;
    expect(capture(document.getElementById("target")!, AT).source, value).toBeUndefined();
  }
});

test("a pin on a primitive's own node resolves to the composition that drew it", () => {
  // What the build leaves: the composition's elements stamped, the primitive's not.
  document.body.innerHTML = `
    <section ${SOURCE_ATTRIBUTE}="packages/components/src/compositions/ActiveJobsList/ActiveJobsList.tsx:224">
      <h2 id="own" ${SOURCE_ATTRIBUTE}="packages/components/src/compositions/ActiveJobsList/ActiveJobsList.tsx:228">Needs you</h2>
      <button class="armada-button"><span id="inside">Dispatch</span></button>
    </section>`;
  expect(capture(document.getElementById("inside")!, AT).source).toEqual({
    file: "packages/components/src/compositions/ActiveJobsList/ActiveJobsList.tsx",
    line: 224,
  });
  expect(capture(document.getElementById("own")!, AT).source).toEqual({
    file: "packages/components/src/compositions/ActiveJobsList/ActiveJobsList.tsx",
    line: 228,
  });
});

test("a note names the places around the element and the heading above it, innermost first, and what is selected", () => {
  document.body.innerHTML = `
    <main aria-label="Usage">
      <div role="tablist" aria-label="Burn by">
        <button role="tab" aria-selected="false">Session</button><button role="tab" aria-selected="true">Bash</button>
      </div>
      <section aria-label="Burn">
        <h2>Where the burn went</h2>
        <ul aria-label="Burn by Bash command">
          <li aria-label="cargo test, 41.0% of the burn">cargo test</li>
          <li id="row" aria-label="Other commands, 16.1% of the burn">Other commands</li>
        </ul>
      </section>
    </main>
    <div data-armada-annotate=""><button aria-pressed="true">Pin</button></div>`;
  const note = capture(document.getElementById("row")!, AT);
  expect(note.nearby).toEqual(["Burn by Bash command", "Where the burn went", "Burn", "Usage"]);
  expect(note.selected).toEqual(['tab "Bash"']);
});

test("a note reads current, pressed, checked, expanded and open, never `aria-current=\"false\"`", () => {
  document.body.innerHTML = `
    <a href="/" aria-current="page">Usage</a><a href="/x" aria-current="false">Cockpit</a>
    <button aria-pressed="true" aria-label="Mono"></button>
    <input type="checkbox" aria-checked="true" aria-label="Autoplay">
    <button aria-expanded="true">More</button>
    <details open><summary>Why</summary>Because</details>
    <dialog open aria-label="Settings"><p id="inside">Here</p></dialog>`;
  const note = capture(document.getElementById("inside")!, AT);
  expect(note.selected).toEqual(['link "Usage"', 'button "Mono"', 'checkbox "Autoplay"', 'button "More"', 'group "Why"', 'dialog "Settings"']);
  expect(note.nearby).toEqual(["Settings"]);
});

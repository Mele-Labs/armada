// The note box over a viewer: Enter holds a note, Shift+Enter is a new line, ⌘Enter sends everything held and what is
// being typed as one message, Esc closes and keeps the draft. The Job picker is there only where the viewer is in no Job.

import { useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { useViewerNotes, ViewerNotesFrom } from "@armada/components";
import type { NoteAbout, ViewerNotesHost, ViewerNotesScope } from "@armada/components";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];
let scopes = 0;

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const FIRST: NoteAbout = { kind: "file", path: "crates/fleet/src/retry.rs", from: 12, to: 12, quote: "let a = 1;" };
const SECOND: NoteAbout = { kind: "file", path: "crates/fleet/src/backoff.rs", from: 3, to: 5, quote: "a\nb\nc" };

/** A viewer that points at `FIRST` or `SECOND` when pressed, as a file viewer does on a right press. */
function Viewer({ scope }: { scope: ViewerNotesScope }) {
  const from = useRef<HTMLDivElement>(null);
  const viewer = useViewerNotes(scope, from);
  return (
    <div ref={from} role="group" tabIndex={0} aria-label="Viewer" onKeyDown={viewer.onKeyDown}>
      <button type="button" onClick={() => viewer.menu({ x: 40, y: 40 }, FIRST)}>
        Point at the first
      </button>
      <button type="button" onClick={() => viewer.menu({ x: 40, y: 40 }, SECOND)}>
        Point at the second
      </button>
      <output aria-label="Marks">{viewer.marks.map((one) => `${String(one.n)}:${one.text}`).join("|")}</output>
      {viewer.layer}
    </div>
  );
}

function draw(host: ViewerNotesHost, job?: string) {
  scopes += 1;
  const scope: ViewerNotesScope = { key: `box-test-${String(scopes)}`, ...(job === undefined ? {} : { job }) };
  const element = document.body.appendChild(document.createElement("div"));
  const root = createRoot(element);
  roots.push({ root, host: element });
  root.render(
    <ViewerNotesFrom host={host}>
      <Viewer scope={scope} />
    </ViewerNotesFrom>,
  );
}

const send = vi.fn<ViewerNotesHost["send"]>();
const jobs = [
  { id: "job-1", title: "Retry tuning" },
  { id: "job-2", title: "Lint sweep" },
];
beforeEach(() => {
  send.mockReset();
  send.mockResolvedValue(true);
});

/** The menu a press on the viewer opens, then Add note, which opens the box. */
async function addNote(pointing: "first" | "second") {
  await userEvent.click(page.getByRole("button", { name: `Point at the ${pointing}` }));
  await userEvent.click(page.getByRole("menuitem", { name: /Add note/ }));
}

const note = () => page.getByRole("textbox", { name: "Note" });
const sendButton = () => page.getByRole("button", { name: /^Send(?! to)/ });

test("Enter holds a note without sending it", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Enter}");

  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
  await expect.element(page.getByRole("status", { name: "Marks" })).toHaveTextContent("1:Why twelve?");
  expect(send).not.toHaveBeenCalled();
});

test("Shift+Enter is a new line in the note and does not hold it", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "one{Shift>}{Enter}{/Shift}two");

  await expect.element(note()).toHaveValue("one\ntwo");
  await expect.element(page.getByRole("dialog")).toBeInTheDocument();
  expect(send).not.toHaveBeenCalled();
});

test("⌘Enter sends the held notes and the one being typed as a single message, in order", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Enter}");
  await addNote("second");
  // The box reopening full of the held note is its own test below; this one is about what is sent.
  await userEvent.clear(note());
  await userEvent.type(note(), "Cap this{Meta>}{Enter}{/Meta}");

  await expect.poll(() => send.mock.calls.length).toBe(1);
  const [job, message] = send.mock.calls[0]!;
  expect(job).toBe("job-1");
  expect(message.text).toBe(
    ["1. File crates/fleet/src/retry.rs:12", "   > let a = 1;", "   Why twelve?", "", "2. File crates/fleet/src/backoff.rs:3-5", "   > a", "   > b", "   > c", "   Cap this"].join("\n"),
  );
  expect(message.pictures).toEqual([]);
});

test("the box opens empty for the next note once one is held", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Enter}");
  await addNote("second");

  await expect.element(note()).toHaveValue("");
});

test("the box opens empty once the notes are sent", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Meta>}{Enter}{/Meta}");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  await addNote("second");

  await expect.element(note()).toHaveValue("");
});

test("a sent note is gone from the viewer", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Enter}");
  await expect.element(page.getByRole("status", { name: "Marks" })).toHaveTextContent("1:Why twelve?");
  await addNote("second");
  await userEvent.clear(note());
  await userEvent.type(note(), "Cap this{Meta>}{Enter}{/Meta}");

  await expect.poll(() => send.mock.calls.length).toBe(1);
  await expect.element(page.getByRole("status", { name: "Marks" })).toHaveTextContent("");
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("⌘Enter on the viewer sends the held notes without opening the box", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Enter}");
  await userEvent.click(page.getByRole("group", { name: "Viewer" }));
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");

  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]![1].text).toBe("1. File crates/fleet/src/retry.rs:12\n   > let a = 1;\n   Why twelve?");
  await expect.element(page.getByRole("status", { name: "Marks" })).toHaveTextContent("");
});

test("a message that did not arrive leaves the notes held", async () => {
  send.mockResolvedValue(false);
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?{Meta>}{Enter}{/Meta}");

  await expect.poll(() => send.mock.calls.length).toBe(1);
  await expect.element(page.getByRole("status", { name: "Marks" })).toHaveTextContent("1:Why twelve?");
});

test("Esc closes the box and keeps what was typed for the next open", async () => {
  draw({ jobs: [], send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "half a thought");
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
  expect(send).not.toHaveBeenCalled();

  await addNote("first");
  await expect.element(note()).toHaveValue("half a thought");
});

test("with no open Job the box offers the Jobs to pick, and Send waits for one", async () => {
  draw({ jobs, send });
  await addNote("first");
  await userEvent.type(note(), "Why twelve?");

  await expect.element(page.getByRole("button", { name: "Send to a Job" })).toBeVisible();
  await expect.element(sendButton()).toBeDisabled();

  await userEvent.click(page.getByRole("button", { name: "Send to a Job" }));
  await userEvent.click(page.getByRole("menuitem", { name: "Lint sweep" }));
  await expect.element(page.getByRole("button", { name: "Send to Lint sweep" })).toBeVisible();
  await expect.element(sendButton()).toBeEnabled();

  await userEvent.click(sendButton());
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]![0]).toBe("job-2");
});

test("in an open Job there is no Job to pick, and the note goes to that Job", async () => {
  draw({ jobs, send }, "job-1");
  await addNote("first");
  await userEvent.type(note(), "Why twelve?");

  await expect.element(page.getByRole("button", { name: /Send to/ })).not.toBeInTheDocument();
  await expect.element(sendButton()).toBeEnabled();

  await userEvent.click(sendButton());
  await expect.poll(() => send.mock.calls.length).toBe(1);
  expect(send.mock.calls[0]![0]).toBe("job-1");
});

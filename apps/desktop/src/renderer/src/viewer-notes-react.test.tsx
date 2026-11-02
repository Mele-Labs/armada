// A reaction is a note held at once: no box opens, it joins the held notes, and ⌘Enter sends it with them to the Job.

import { useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { useViewerNotes } from "@armada/components";
import type { NoteAbout } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import "./styles/index.css";
import { ViewerNotesOver } from "./viewer-notes";

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

const ABOUT: NoteAbout = {
  kind: "document",
  place: {
    page: "Pricing",
    revision: 3,
    section: "Still open",
  },
  target: {
    kind: "row",
    table: "Table",
    row: "Thresholds | Too tight",
  },
};

let viewers = 0;

function Viewer() {
  const from = useRef<HTMLDivElement>(null);
  const key = useRef(`react-test-row-${String((viewers += 1))}`);
  const viewer = useViewerNotes(
    {
      key: key.current,
      job: "job-1",
    },
    from,
  );
  return (
    <div ref={from} tabIndex={0} onKeyDown={viewer.onKeyDown}>
      <button type="button" onClick={() => viewer.react(ABOUT, "👍 Yes to this")}>
        React to the row
      </button>
      <button type="button" onClick={() => viewer.react(ABOUT, "  ")}>
        React with nothing
      </button>
      <span>{viewer.marks.length} held</span>
      {viewer.layer}
    </div>
  );
}

function draw(send: SessionsDraft["send"]) {
  // One list for every read: the draft is read as a store snapshot, and a new array each time never settles.
  const sessions = [
    {
      id: "job-1",
      title: "Pricing",
    } as unknown as Session,
  ];
  const draft = {
    get: () => sessions,
    subscribe: () => () => undefined,
    send,
  } as unknown as SessionsDraft;
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  root.render(
    <ViewerNotesOver draft={draft}>
      <Viewer />
    </ViewerNotesOver>,
  );
}

test("a reaction is held with no box, and goes to the Job with the notes", async () => {
  const send = vi.fn(async () => true);
  draw(send as unknown as SessionsDraft["send"]);
  await userEvent.click(
    page.getByRole("button", {
      name: "React to the row",
    }),
  );

  await expect.element(page.getByText("1 held")).toBeVisible();
  expect(document.querySelector(".armada-viewer-notes__layer")).toBeNull();

  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.poll(() => send.mock.calls.length).toBe(1);
  const [job, message] = send.mock.calls[0] as unknown as [
    string,
    {
      text: string;
    },
  ];
  expect(job).toBe("job-1");
  expect(message.text).toContain("👍 Yes to this");
});

test("a reaction with no words holds nothing", async () => {
  draw(vi.fn(async () => true) as unknown as SessionsDraft["send"]);
  await userEvent.click(
    page.getByRole("button", {
      name: "React with nothing",
    }),
  );
  await expect.element(page.getByText("0 held")).toBeVisible();
});

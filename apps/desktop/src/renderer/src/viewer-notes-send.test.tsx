// Notes sent to a Job through the Sessions draft, as the Session composer sends: the notes as the message's text, each
// picture's crop as a file along with it, and only the Jobs that can still be told something offered.

import { useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import { useViewerNotes } from "@armada/components";
import type { NoteAbout } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import "./styles/index.css";
import { ViewerNotesOver } from "./viewer-notes";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

// One transparent pixel.
const PNG = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==";
const ABOUT: NoteAbout = { kind: "image", name: "shot.png", path: "/tmp/shot.png", crop: PNG };

const session = (id: string, title: string, dead?: true): Session => ({ id, title, ...(dead === undefined ? {} : { dead: {} }) }) as unknown as Session;

// A key of its own for each test: the Job picked for a viewer is held with it.
let viewers = 0;

function Viewer() {
  const from = useRef<HTMLDivElement>(null);
  const key = useRef(`send-test-picture-${String((viewers += 1))}`);
  const viewer = useViewerNotes({ key: key.current }, from);
  return (
    <div ref={from} tabIndex={0}>
      <button type="button" onClick={() => viewer.menu({ x: 40, y: 40 }, ABOUT)}>
        Point at the picture
      </button>
      {viewer.layer}
    </div>
  );
}

function draw(sessions: Session[], send: SessionsDraft["send"]) {
  const draft = { get: () => sessions, subscribe: () => () => undefined, send } as unknown as SessionsDraft;
  const host = document.body.appendChild(document.createElement("div"));
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <ViewerNotesOver draft={draft}>
      <Viewer />
    </ViewerNotesOver>,
  );
}

async function openBox() {
  await userEvent.click(page.getByRole("button", { name: "Point at the picture" }));
  await userEvent.click(page.getByRole("menuitem", { name: /Add note/ }));
}

test("a picture's crop goes with the message as the file the text names", async () => {
  const send = vi.fn(async () => true);
  draw([session("job-1", "Retry tuning")], send as unknown as SessionsDraft["send"]);
  await openBox();
  await userEvent.click(page.getByRole("button", { name: "Send to a Job" }));
  await userEvent.click(page.getByRole("menuitem", { name: "Retry tuning" }));
  await userEvent.type(page.getByRole("textbox", { name: "Note" }), "Too blurry{Meta>}{Enter}{/Meta}");

  await expect.poll(() => send.mock.calls.length).toBe(1);
  const [job, message] = send.mock.calls[0] as unknown as [string, { text: string; files: { name: string; src: string; file: File }[]; sketches: unknown[]; tags: unknown[] }];
  expect(job).toBe("job-1");
  expect(message.text).toBe("1. Picture /tmp/shot.png, attached as picture-1.png\n   Too blurry");
  expect(message.files).toHaveLength(1);
  expect(message.files[0]!.name).toBe("picture-1.png");
  expect(message.files[0]!.src).toBe(PNG);
  expect(message.files[0]!.file.name).toBe("picture-1.png");
  expect(message.files[0]!.file.type).toBe("image/png");
  expect(message.files[0]!.file.size).toBeGreaterThan(0);
  expect(message.sketches).toEqual([]);
  expect(message.tags).toEqual([]);
});

test("the notes stay held when the draft does not take the message", async () => {
  const send = vi.fn(async () => false);
  draw([session("job-1", "Retry tuning")], send as unknown as SessionsDraft["send"]);
  await openBox();
  await userEvent.click(page.getByRole("button", { name: "Send to a Job" }));
  await userEvent.click(page.getByRole("menuitem", { name: "Retry tuning" }));
  await userEvent.type(page.getByRole("textbox", { name: "Note" }), "Too blurry{Meta>}{Enter}{/Meta}");

  await expect.poll(() => send.mock.calls.length).toBe(1);
  await expect.element(page.getByRole("dialog")).toBeInTheDocument();
});

test("only the Jobs that can still be told something are offered", async () => {
  draw([session("job-1", "Retry tuning"), session("job-2", "Lint sweep", true)], vi.fn() as unknown as SessionsDraft["send"]);
  await openBox();
  await userEvent.click(page.getByRole("button", { name: "Send to a Job" }));

  await expect.element(page.getByRole("menuitem", { name: "Retry tuning" })).toBeVisible();
  expect(page.getByRole("menuitem", { name: "Lint sweep" }).elements()).toHaveLength(0);
});

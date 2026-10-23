// The Artifacts layer's graph, in a browser with real keys: V switches the view, the arrows walk the nodes, a full stop steps into a node's versions,
// Enter opens, the search dims, and the history is read when the graph opens and when the ledger moves. The Sessions are a stub draft.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { userEvent } from "vitest/browser";
import type { Session, SessionAttachment, SessionRow, SessionsDraft } from "@armada/screens/src/draft/sessions";

import "../styles/index.css";
import { SessionsFrom } from "../sessions-draft";
import { closeLayer } from "./layers";
import { ArtifactsLayer } from "./SessionLayers";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  closeLayer();
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

type Versions = NonNullable<Session["versions"]>;

const SKETCH: Extract<SessionAttachment, { kind: "sketch" }> = { kind: "sketch", id: "sketch:Reads", title: "Reads", by: "agent", drawing: { nodes: [], edges: [] } };
const STARTED: SessionAttachment = { kind: "started_to", id: "c1", group: "g1" };
const LINK: SessionAttachment = { kind: "link", url: "https://example.com/rfc", by: "you" };

const rows: SessionRow[] = [{ id: "m1", at: "2026-10-22T10:00:00Z", kind: "message", from: { kind: "you" }, text: "Read the first note" }];
const plan = { version: 1, scope: [], approved: true, after: "m1", groups: [{ id: "g1", agent: "rust-engineer", title: "Draw it", paths: [], tasks: [{ id: "t1", text: "Draw the reads", state: "done" as const }] }] };

const make = (attachments: SessionAttachment[]): Session => ({ id: "j1", title: "Draw the reads", attachments, rows, turn: { state: "idle" }, work: { plan } }) as unknown as Session;
const child = { id: "c1", title: "Run the report", attachments: [{ kind: "started_by", id: "j1", group: "g1" }], rows: [], turn: { state: "idle" }, standing: "idle" } as unknown as Session;

const HISTORY: Versions = [
  { at: "2026-10-22T10:01:00Z", attachment: LINK, from: { message: "m1" } },
  { at: "2026-10-22T10:05:00Z", attachment: SKETCH, from: { message: "m1" } },
  { at: "2026-10-22T10:20:00Z", attachment: { ...SKETCH, title: "Reads again" }, from: { task: "t1" } },
  { at: "2026-10-22T10:30:00Z", attachment: STARTED, from: { task: "t1" } },
];

/** The layer over a stub draft whose history is `history`; `rerender` hands it another Session. */
function mount(history: SessionsDraft["history"], opened: { session: string[]; sketch: [string, string | undefined][] } = { session: [], sketch: [] }) {
  const host = document.createElement("div");
  host.style.cssText = "width:900px;height:600px";
  document.body.append(host);
  const root = createRoot(host);
  const others = [child];
  const draft = { get: () => others, subscribe: () => () => undefined, ...(history === undefined ? {} : { history }) } as unknown as SessionsDraft;
  const render = (session: Session) =>
    act(() =>
      root.render(
        <SessionsFrom held={draft}>
          <ArtifactsLayer session={session} entries={[]} onOpenSession={(id) => opened.session.push(id)} onOpenSketch={(id, at) => opened.sketch.push([id, at])} />
        </SessionsFrom>,
      ),
    );
  roots.push({ root, host });
  const first = make([LINK, SKETCH, STARTED]);
  render(first);
  return { host, render, opened, first };
}

const graphOf = (host: HTMLElement) => host.querySelector<HTMLElement>('[role="group"][aria-label="Artifact graph"]');
const node = (host: HTMLElement, name: string) => host.querySelector<HTMLButtonElement>(`button[data-node][aria-label="${name}"]`)!;
const focused = () => document.activeElement?.getAttribute("aria-label");
const settled = () => act(async () => void (await new Promise((done) => setTimeout(done, 50))));

/** Opens the graph the way a person does: Tab from the search field to the switch, then V. */
async function openGraph(host: HTMLElement) {
  await userEvent.keyboard("{Tab}");
  await userEvent.keyboard("v");
  await vi.waitFor(() => expect(graphOf(host)).not.toBeNull());
  await settled();
}

test("V switches the list to the graph and back, but is a letter in the search field", async () => {
  const { host } = mount(async () => HISTORY);
  const field = host.querySelector<HTMLInputElement>('[aria-label="Search artifacts"]')!;
  expect(document.activeElement).toBe(field);
  await userEvent.keyboard("v");
  expect(field.value).toBe("v");
  expect(graphOf(host)).toBeNull();
  field.select();
  await userEvent.keyboard("{Backspace}{Tab}v");
  await vi.waitFor(() => expect(graphOf(host)).not.toBeNull());
  expect(host.querySelector(".sv-layer__group")).toBeNull();
  await userEvent.keyboard("v");
  await vi.waitFor(() => expect(graphOf(host)).toBeNull());
  expect(host.querySelector(".sv-layer__group")).not.toBeNull();
});

test("the arrows move between nodes and Enter opens the one focused", async () => {
  const { host, opened } = mount(async () => HISTORY);
  await openGraph(host);
  act(() => node(host, "Message, Read the first note").focus());
  await userEvent.keyboard("{ArrowRight}");
  expect(focused()).toBe("Plan, version 1");
  await userEvent.keyboard("{ArrowRight}");
  expect(focused()).toBe("Group, Draw it");
  await userEvent.keyboard("{ArrowLeft}{ArrowLeft}");
  expect(focused()).toBe("Message, Read the first note");
  act(() => node(host, "Linked Job, Run the report").focus());
  await userEvent.keyboard("{Enter}");
  expect(opened.session).toEqual(["c1"]);
});

test("a full stop steps into a node's versions and Enter on a mark opens that version", async () => {
  const { host, opened } = mount(async () => HISTORY);
  await openGraph(host);
  act(() => node(host, "Sketch, Reads").focus());
  await userEvent.keyboard(".");
  expect(focused()).toBe("Drawn, Reads");
  await userEvent.keyboard(".");
  expect(focused()).toBe("Redrawn, Reads");
  await userEvent.keyboard("{Enter}");
  expect(opened.sketch).toEqual([["sketch:Reads", "2026-10-22T10:20:00Z"]]);
  await userEvent.keyboard("{Escape}");
  expect(focused()).toBe("Sketch, Reads");
});

test("a search dims the nodes it does not match and leaves them there", async () => {
  const { host } = mount(async () => HISTORY);
  await openGraph(host);
  act(() => host.querySelector<HTMLInputElement>('[aria-label="Search artifacts"]')!.focus());
  await userEvent.keyboard("rfc");
  const dim = (name: string) => node(host, name).closest<HTMLElement>(".armada-artifact-node")!.dataset.dim;
  await vi.waitFor(() => expect(dim("Sketch, Reads")).toBe("true"));
  expect(dim("Link, example.com/rfc")).toBeUndefined();
  expect(node(host, "Plan, version 1")).not.toBeNull();
});

test("the history is read when the graph opens and not before, and again when the ledger moves, never otherwise", async () => {
  const read = vi.fn(async () => HISTORY);
  const { host, render, first } = mount(read);
  await settled();
  expect(read).not.toHaveBeenCalled();
  await openGraph(host);
  expect(read).toHaveBeenCalledTimes(1);
  expect(read).toHaveBeenCalledWith("j1");
  // The Session moved but its ledger did not: nothing new to read.
  render({ ...first, title: "Draw the reads again" });
  await settled();
  expect(read).toHaveBeenCalledTimes(1);
  // A new attachment is a version to read.
  render({ ...first, attachments: [...first.attachments, { kind: "link", url: "https://example.com/more", by: "agent" }] });
  await settled();
  expect(read).toHaveBeenCalledTimes(2);
});

test("a history that could not be read leaves the graph drawn from the ledger, with no marks and nothing said", async () => {
  const { host } = mount(async () => undefined);
  await openGraph(host);
  expect(node(host, "Sketch, Reads")).not.toBeNull();
  expect(node(host, "Linked Job, Run the report")).not.toBeNull();
  expect(host.querySelector('[aria-label^="Versions of"]')).toBeNull();
  expect(host.textContent).not.toMatch(/could not|failed|error/i);
});

test("a draft that serves no history draws the graph the same way", async () => {
  const { host } = mount(undefined);
  await openGraph(host);
  expect(node(host, "Sketch, Reads")).not.toBeNull();
  expect(host.querySelector('[aria-label^="Versions of"]')).toBeNull();
});

test("the versions show once the history has been read", async () => {
  const { host } = mount(async () => HISTORY);
  await openGraph(host);
  await vi.waitFor(() => expect(host.querySelector('[aria-label="Versions of Reads"]')).not.toBeNull());
});

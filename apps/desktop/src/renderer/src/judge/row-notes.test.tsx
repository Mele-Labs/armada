// A Judge on an artifact with no view of its own: its notes stand under its row in the Job's artifacts, past the Commands' cap, and a line in the thread leads to them. The whole
// Job's lead to the Judge tab. Browser: a rendering is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { Terminal } from "lucide-react";
import type { JudgeReading } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { ArtifactRow } from "../session-view/artifacts";
import { closeLayer, layerOver } from "../session-view/layers";
import { Groups } from "../session-view/SessionLayers";
import { openAtNotes } from "./artifacts";
import { notedUnderRow } from "./notes";
import { provideJudge, putReadings, type JudgeArtifact } from "./store";
import "../styles/index.css";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const JOB = "job-1";
const SESSION = {
  id: JOB,
  title: "Switch sign-in",
} as Session;
const rowOf = (at: number): ArtifactRow =>
  ({
    key: `ran:${at}`,
    title: `command ${at}`,
    said: "Command",
    Glyph: Terminal,
    open: () => undefined,
  }) as ArtifactRow;
const GROUPS = [
  {
    id: "command",
    label: "Commands",
    rows: Array.from(
      {
        length: 12,
      },
      (_, at) => rowOf(at),
    ),
  },
];
const artifact = (at: number): JudgeArtifact => ({
  kind: "command",
  id: String(at),
  title: `command ${at}`,
});
const reading = (at: number): JudgeReading => ({
  id: `r${at}`,
  artifact: artifact(at),
  prompt: "",
  state: "done",
  asked_at: "2026-10-31T14:00:00Z",
  read: false,
  notes: [
    {
      id: "n1",
      claim: "It printed two warnings.",
      where: `command ${at}`,
    },
  ],
});

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
beforeEach(() =>
  provideJudge({
    ask: vi.fn(),
    send: vi.fn(),
    dismiss: vi.fn(),
    markRead: vi.fn(),
  }),
);
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  provideJudge(undefined);
  act(() => closeLayer());
});

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() =>
    root.render(
      <ul>
        <Groups groups={GROUPS} session={SESSION} />
      </ul>,
    ),
  );
  roots.push({
    root,
    host,
  });
  return host;
}

const note = (host: HTMLElement) =>
  host.querySelector<HTMLElement>('[role="group"][aria-label^="Finding: It printed two warnings"]');
const rowNamed = (host: HTMLElement, name: string) =>
  host.querySelector(`button[aria-label="${name}"]`);

test("a note stands under the row it is about", () => {
  act(() =>
    putReadings(JOB, [
      reading(1),
    ]),
  );
  const host = mount();
  const found = note(host)!;
  expect(
    rowNamed(host, "command 1")!.compareDocumentPosition(found) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  expect(
    found.compareDocumentPosition(rowNamed(host, "command 2")!) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
});

test("a row past the Commands' cap is drawn when it holds notes, and its neighbour stays folded", () => {
  act(() =>
    putReadings(JOB, [
      reading(11),
    ]),
  );
  const host = mount();
  expect(rowNamed(host, "command 11")).not.toBeNull();
  expect(rowNamed(host, "command 10")).toBeNull();
  expect(note(host)).not.toBeNull();
});

test("what has a view of its own draws no notes under a row", () => {
  for (const kind of [
    "job",
    "plan",
    "pull_request",
    "doc",
  ] as const)
    expect(
      notedUnderRow({
        kind,
      }),
      kind,
    ).toBe(false);
  for (const kind of [
    "commit",
    "file",
    "page",
    "sketch",
    "command",
    "subagent",
    "issue",
    "note",
  ] as const)
    expect(
      notedUnderRow({
        kind,
      }),
      kind,
    ).toBe(true);
});

test("a whole Job's reading opens at the Judge tab, and a command's at the Job's artifacts", () => {
  act(() =>
    openAtNotes(SESSION, {
      kind: "job",
      id: JOB,
    }),
  );
  expect(layerOver(JOB)).toBe("judge");
  act(() => closeLayer());
  act(() => openAtNotes(SESSION, artifact(1)));
  expect(layerOver(JOB)).toBe("artifacts");
});

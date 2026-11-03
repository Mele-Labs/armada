// A ticket row's right press offers the Ticket lifecycle's states, ticks the one its status reads as, and teaches the status as the one picked;
// a refusal is the window's toast, and a row wears the glyph of the Ticket state its status reads as. Browser: a rendering is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { AppliedLifecycle, Outcome } from "@armada/protocol";

import "./styles/index.css";
import { IssueRow, type Issue } from "./session-view/IssuesBlock";
import { ticketWordOf } from "./ticket-word-menu";

vi.mock("./shared-panel/store", () => ({
  openInPanel: vi.fn(() => true),
}));

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const MANIFEST = "tidewater";
const WORD = "Ready for QA";

const ticketLifecycle = (words: Record<string, string[]>): AppliedLifecycle => ({
  lifecycle: {
    id: "ticket",
    name: "Ticket",
    scope: {
      kind: "everywhere",
    },
    states: [
      "todo",
      "in_progress",
      "in_review",
      "closed",
    ].map((id) => ({
      id,
      label: {
        todo: "To do",
        in_progress: "In progress",
        in_review: "In review",
        closed: "Closed",
      }[id]!,
      read: {
        source: "tracker" as const,
        words: words[id] ?? [],
      },
    })),
    transitions: [],
    ends: [
      "closed",
    ],
  } as AppliedLifecycle["lifecycle"],
  kept: {
    rank: {
      reach: "everywhere",
      source: "armada",
    },
  },
});

let taught: Record<string, string[]> = {};
const listLifecycles = vi.fn(async (_manifestId: string) => ({
  ok: true as const,
  value: {
    manifest_id: MANIFEST,
    lifecycles: [
      ticketLifecycle(taught),
    ],
    left_out: [],
  },
}));
const teachTicketWord = vi.fn(
  async (_body: { manifest_id: string; word: string; state: string }) =>
    ({
      ok: true,
      value: {
        saved: {},
        jobs: [],
      },
    }) as
      | {
          ok: true;
          value: unknown;
        }
      | {
          ok: false;
          outcome: Outcome;
        },
);
const raised = vi.fn();

let root: Root | undefined;
let host: HTMLElement | undefined;

const issue = (over: Partial<Issue> = {}): Issue => ({
  kind: "issue",
  number: "ABC-12",
  title: "Keep the tide tables a day",
  address: "https://tracker.example/ABC-12",
  state: "open",
  status: WORD,
  ...over,
});

const draw = (one: Issue) =>
  act(async () => root!.render(<IssueRow issue={one} manifestId={MANIFEST} />));

beforeEach(async () => {
  taught = {};
  listLifecycles.mockClear();
  teachTicketWord.mockClear();
  raised.mockClear();
  (
    window as unknown as {
      armada: unknown;
    }
  ).armada = {
    listLifecycles,
    teachTicketWord,
  };
  window.addEventListener("armada:raise-outcome", raised);
  host = document.body.appendChild(document.createElement("div"));
  root = createRoot(host);
});

afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
  window.removeEventListener("armada:raise-outcome", raised);
});

const row = () => host!.querySelector<HTMLElement>("button.sv-row")!;
const rightPress = () =>
  act(
    async () =>
      void row().dispatchEvent(
        new MouseEvent("contextmenu", {
          bubbles: true,
          cancelable: true,
          clientX: 20,
          clientY: 20,
        }),
      ),
  );
const items = () => [
  ...document.querySelectorAll<HTMLElement>('[role="menuitem"]'),
];
const item = (name: string) => items().find((one) => one.textContent === name);
const mark = () => host!.querySelector<HTMLElement>('[role="img"]')!;

test("a right press on a ticket row opens a menu headed with its status and the four Ticket states, none selected when no state lists the word", async () => {
  await draw(issue());
  await rightPress();
  expect(document.body.textContent).toContain("ABC-12 is Ready for QA. In Armada that is");
  expect(items().map((one) => one.textContent)).toEqual([
    "To do",
    "In progress",
    "In review",
    "Closed",
  ]);
  expect(items().filter((one) => one.hasAttribute("aria-current"))).toEqual([]);
});

test("the state whose tracker words list the word is the one selected", async () => {
  taught = {
    in_review: [
      WORD,
    ],
  };
  await draw(issue());
  await rightPress();
  expect(items().filter((one) => one.hasAttribute("aria-current"))).toEqual([
    item("In review"),
  ]);
});

test("picking In review teaches the word as in_review in the row's repository", async () => {
  await draw(issue());
  await rightPress();
  await act(async () => item("In review")!.click());
  expect(teachTicketWord).toHaveBeenCalledTimes(1);
  expect(teachTicketWord).toHaveBeenCalledWith({
    manifest_id: MANIFEST,
    word: WORD,
    state: "in_review",
  });
});

test("a refused teach is raised as the window's toast and ticks no state", async () => {
  const refusal: Outcome = {
    ok: false,
    why: "not_connected",
  };
  teachTicketWord.mockResolvedValueOnce({
    ok: false,
    outcome: refusal,
  });
  await draw(issue());
  await rightPress();
  const reads = listLifecycles.mock.calls.length;
  await act(async () => item("In review")!.click());
  expect(raised).toHaveBeenCalledTimes(1);
  expect((raised.mock.calls[0]![0] as CustomEvent<Outcome>).detail).toEqual(refusal);
  // A taught word reads the lifecycle again; a refused one leaves it as it was.
  expect(listLifecycles).toHaveBeenCalledTimes(reads);
  await rightPress();
  expect(items().filter((one) => one.hasAttribute("aria-current"))).toEqual([]);
});

test("a ticket's word is its tracker's status, else Open or Closed from its state", () => {
  expect(ticketWordOf("Ready for QA", "open")).toBe("Ready for QA");
  expect(ticketWordOf("Ready for QA", "closed")).toBe("Ready for QA");
  expect(ticketWordOf(undefined, "open")).toBe("Open");
  expect(ticketWordOf(undefined, "closed")).toBe("Closed");
  expect(ticketWordOf("  ", "closed")).toBe("Closed");
});

test("the ContextMenu key on a focused row opens the same menu", async () => {
  await draw(issue());
  row().focus();
  await act(async () =>
    row().dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "ContextMenu",
        bubbles: true,
        cancelable: true,
      }),
    ),
  );
  expect(document.body.textContent).toContain("ABC-12 is Ready for QA. In Armada that is");
  expect(items().map((one) => one.textContent)).toEqual([
    "To do",
    "In progress",
    "In review",
    "Closed",
  ]);
});

test("a row whose status reads as In review wears the eye glyph and says so, and an unmapped one keeps its Open mark", async () => {
  taught = {
    in_review: [
      WORD,
    ],
  };
  await draw(issue());
  expect(mark().getAttribute("aria-label")).toBe("Ticket state: In review");
  expect(mark().querySelector("svg.lucide-eye")).not.toBeNull();

  await draw(
    issue({
      status: "Waiting on vendor",
    }),
  );
  expect(mark().getAttribute("aria-label")).toBe("Open");
  expect(mark().querySelector("svg.lucide-eye")).toBeNull();
  expect(mark().querySelector("svg.lucide-ticket")).not.toBeNull();
});

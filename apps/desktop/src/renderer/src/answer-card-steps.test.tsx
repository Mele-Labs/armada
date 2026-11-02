// A waiting item with choices and steps draws its steps, numbered, under the question and above the choices, in the Needs you
// card and on the Cockpit's call card; the choices answer as they did. An item with only one of the two draws as before.

import { LedgerRows, waitingMark } from "@armada/components";
import type { WaitingItem } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { MessageCircleQuestion } from "lucide-react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { CallCard } from "./cockpit/CallCard";
import type { BridgeState } from "../../shared/bridge";
import type { Item } from "./Dashboard";
import { entriesOf } from "./sessions";

import "./styles/index.css";

const AT = "2026-11-02T09:14:02.000Z";
const TITLE = "Ask the other engineer to send back three command outputs";
const STEPS = [
  "Update to a build with #3197",
  "Start a Job in the repository that failed before, and let it work for two turns",
  "In that Job's terminal, run `echo $CLAUDE_CODE_SESSION_ID`, `armada job status` and `armada report`",
  "Paste all three outputs back here",
];
const OPTIONS = [
  {
    label: "Sent",
  },
  {
    label: "Not needed",
  },
];

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

function mount(node: React.ReactNode) {
  const host = document.createElement("div");
  host.style.width = "640px";
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  root.render(node);
}

const item = (extra: Partial<WaitingItem>): WaitingItem => ({
  id: "w1",
  text: TITLE,
  since: AT,
  source: "agent",
  ...extra,
});

const both = item({
  act: {
    kind: "answer",
    target: "",
  },
  options: OPTIONS,
  steps: STEPS,
});

const goes = {
  onGoTo: vi.fn(),
  onOpenLink: vi.fn(),
  onOpenCheck: vi.fn(),
  onOpenLoadout: vi.fn(),
};

/** The item as the Needs you card draws it: `entriesOf` over a Job that waits on it, then the rows the panel and the ledger share. */
function ledger(waiting: WaitingItem) {
  const session = {
    id: "s1",
    attachments: [],
    rows: [],
    turn: {
      state: "idle",
    },
    waitingFor: [
      waiting,
    ],
  } as unknown as Session;
  const answer = vi.fn();
  const read = vi.fn();
  const row = entriesOf(
    session,
    goes,
    read,
    vi.fn(),
    [
      session,
    ],
    vi.fn(),
    vi.fn(),
    answer,
    vi.fn(),
    {
      slots: [],
    },
    undefined,
  ).find((one) => one.kind === "waiting")!;
  mount(<LedgerRows kind="waiting" Glyph={waitingMark(row).Glyph} rows={[row]} />);
  return {
    row,
    answer,
    read,
  };
}

const steps = () => page.getByRole("listitem").elements();
const choice = (name: string) =>
  page.getByRole("radio", {
    name,
    exact: true,
  });
const before = (first: Element, second: Element) =>
  (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;

/** The numbered steps are an ordered list; the choices are a radio group, and the title is the card's own words. */
const list = () => document.querySelector("ol.armada-step-list__list");

test("a question with choices and steps draws the steps numbered above the choices, each command with a copy key", async () => {
  const { row } = ledger(both);

  await expect.element(page.getByText(TITLE)).toBeVisible();
  await expect.element(choice("Sent")).toBeVisible();
  expect(row.steps?.lines).toEqual(STEPS);
  const numbered = list()!;
  expect(numbered).not.toBeNull();
  expect(numbered.querySelectorAll(":scope > li")).toHaveLength(4);
  expect([
    ...numbered.querySelectorAll(".armada-step-list__ordinal"),
  ].map((one) => one.textContent)).toEqual([
    "1",
    "2",
    "3",
    "4",
  ]);
  // The step that names three commands has a copy key for each, under it.
  await expect
    .element(
      page
        .getByRole("button", {
          name: "Copy step 3 command",
        })
        .first(),
    )
    .toBeVisible();
  expect(
    page
      .getByRole("button", {
        name: "Copy step 3 command",
      })
      .elements(),
  ).toHaveLength(3);
  // The title, then the steps, then the choices.
  expect(before(page.getByText(TITLE).element(), numbered)).toBe(true);
  expect(before(numbered, choice("Sent").element())).toBe(true);
  expect(steps().length).toBeGreaterThan(4);
});

test("Sent still answers with choice 0", async () => {
  const { answer } = ledger(both);

  await choice("Sent").click();
  await page
    .getByRole("button", {
      name: "Answer",
    })
    .click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith("w1", {
    choice: 0,
  });

});

test("a number key picks its choice's tile, and Answer sends that choice", async () => {
  const { answer } = ledger(both);

  const second = choice("Not needed");
  await expect.element(second).toBeVisible();
  second.element().focus();
  await userEvent.keyboard("2");
  await expect.element(second).toBeChecked();
  await page
    .getByRole("button", {
      name: "Answer",
    })
    .click();
  await expect.poll(() => answer.mock.calls.length).toBe(1);
  expect(answer).toHaveBeenLastCalledWith("w1", {
    choice: 1,
  });
});

test("an item with only steps draws no choices and no list on its row, and a press opens the steps", async () => {
  const { row, read } = ledger(
    item({
      steps: STEPS,
    }),
  );

  await expect.element(page.getByText(TITLE)).toBeVisible();
  expect(row.options).toBeUndefined();
  expect(row.steps).toBeUndefined();
  expect(row.act).toBe("steps");
  expect(list()).toBeNull();
  expect(document.querySelector('[role="radio"]')).toBeNull();
  row.onOpen?.();
  expect(read).toHaveBeenCalledWith({
    kind: "steps",
    id: "w1",
  });
});

test("an item with only choices draws the choices and no list", async () => {
  const { row } = ledger(
    item({
      act: {
        kind: "answer",
        target: "",
      },
      options: OPTIONS,
    }),
  );

  await expect.element(choice("Sent")).toBeVisible();
  expect(row.steps).toBeUndefined();
  expect(list()).toBeNull();
});

const CALL = (waiting: WaitingItem): Item => ({
  key: "session:s1:w1",
  icon: MessageCircleQuestion,
  kind: "Question",
  title: TITLE,
  fact: TITLE,
  hue: "ask",
  where: "armada",
  body: [],
  acts: () => null,
  waiting: {
    sessionId: "s1",
    item: waiting,
  },
});

function call(waiting: WaitingItem) {
  mount(
    <CallCard
      item={CALL(waiting)}
      now={Date.now()}
      state={
        {
          mergeLines: [],
          repository: "",
          holds: {},
        } as unknown as BridgeState
      }
      hosts={{
        onOpenSession: vi.fn(),
        onOpenLink: vi.fn(),
      }}
      finish={vi.fn()}
      later={vi.fn()}
      dismiss={vi.fn()}
      leaving={undefined}
      from={undefined}
      answering={{
        current: undefined,
      }}
    />,
  );
}

test("the Cockpit's call card draws the steps, numbered, above the choices", async () => {
  call(both);

  await expect.element(page.getByText("Sent", { exact: true }).first()).toBeVisible();
  const numbered = list()!;
  expect(numbered).not.toBeNull();
  expect(numbered.querySelectorAll(":scope > li")).toHaveLength(4);
  expect(
    page
      .getByRole("button", {
        name: "Copy step 3 command",
      })
      .elements(),
  ).toHaveLength(3);
  const tile = document.querySelector(".armada-callcard__answer, .armada-ask-answer");
  expect(tile).not.toBeNull();
  expect(before(numbered, tile!)).toBe(true);
});

test("the Cockpit's call card draws no list for choices alone or for steps alone", async () => {
  call(
    item({
      act: {
        kind: "answer",
        target: "",
      },
      options: OPTIONS,
    }),
  );
  await expect.element(page.getByText("Not needed").first()).toBeVisible();
  expect(list()).toBeNull();

  call(
    item({
      steps: STEPS,
    }),
  );
  await expect.element(page.getByText(TITLE).last()).toBeVisible();
  expect(document.querySelectorAll("ol.armada-step-list__list")).toHaveLength(0);
});

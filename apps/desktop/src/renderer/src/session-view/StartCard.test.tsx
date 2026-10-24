// The start card and its keys. A Job dispatched from the sheet waits on this card before any agent moves: ↵ starts it, ↑ ↓ move between rows,
// ← → change a row's choice, Esc puts a title edit back. The draft is a fake that answers a Kind change as Fleet does, by working the standing
// landing out again for the new kind, so the card is drawn from what Fleet would send.

import { forgetAllKept } from "@armada/components";
import type { DeliveryChoice } from "@armada/protocol";
import type { Session, SessionsDraft, WorkflowChoice } from "@armada/screens/src/draft/sessions";
import { repository } from "@armada/screens/src/fixtures/repository";
import { useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { SessionsFrom } from "../sessions-draft";
import { StartCard } from "./StartCard";

const ARMADA = repository();

const KINDS: readonly WorkflowChoice[] = [
  { id: "feature", name: "Feature", steps: [] },
  { id: "bug", name: "Bug", steps: [] },
  { id: "review", name: "Review", steps: [] },
];
/** What he usually picks for each kind; a review has no landing step. */
const USUAL: Readonly<Record<string, DeliveryChoice>> = { feature: "auto_land", bug: "open_pr" };

const startFor = (kind: string | null, named?: DeliveryChoice): NonNullable<Session["start"]> => {
  const usual = kind === null ? undefined : USUAL[kind];
  return {
    asksDelivery: kind !== "review",
    ...(named !== undefined ? { picked: { choice: named, from: "named" as const } } : usual === undefined ? {} : { picked: { choice: usual, from: "usual" as const } }),
  };
};

const jobOn = (start: Session["start"], extra: Partial<Session> = {}): Session => ({
  id: "s1",
  title: "Retry the store client",
  manifestId: ARMADA.manifest!.id,
  turn: { state: "idle" },
  rows: [],
  attachments: [],
  model: "opus",
  effort: "high",
  start,
  work: { proposal: { workflow: "feature", picked: [] } },
  ...extra,
});

const begin = vi.fn(async (_id: string, _delivery?: DeliveryChoice) => true);
const rename = vi.fn();
const tune = vi.fn();
const unpick = vi.fn();
const choose = vi.fn();

let change: (next: Session) => void = () => undefined;

/** The draft as far as the card reads it. A Kind change is answered as Fleet answers it: the standing landing is worked out again, and the card is sent the record. */
/** One list, since the view reads Sessions through `useSyncExternalStore` and a new array each read never settles. */
const none: never[] = [];
const draft = {
  get: () => none,
  subscribe: () => () => undefined,
  begin,
  rename,
  tune,
  models: ["opus", "sonnet"],
  efforts: ["low", "high"],
  defaultModel: "opus",
  work: {
    workflows: KINDS,
    choose: (id: string, kind: string | null) => {
      choose(id, kind);
      change(jobOn(startFor(kind), { work: { proposal: { workflow: kind, picked: [] } } }));
    },
    unpick,
  },
} as unknown as SessionsDraft;

function Card({ first }: { first: Session }) {
  const [session, setSession] = useState(first);
  change = setSession;
  return <StartCard session={session} start={session.start!} repositories={[ARMADA]} />;
}

let mounted: { root: Root; host: HTMLElement } | undefined;

function show(session: Session) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted = { root, host };
  root.render(
    <SessionsFrom held={draft}>
      <Card first={session} />
    </SessionsFrom>,
  );
}

beforeEach(() => {
  forgetAllKept();
  for (const one of [begin, rename, tune, unpick, choose]) one.mockClear();
});
afterEach(() => {
  mounted?.root.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

const title = () => page.getByRole("textbox", { name: "Title" });
const kind = () => page.getByRole("combobox", { name: "Kind" });
const model = () => page.getByRole("combobox", { name: "Model" });
const radio = (name: string) => page.getByRole("radio", { name });
const startButton = () => page.getByRole("button", { name: "Start" });

/** Whether a tooltip bubble with these words is showing. */
const bubble = (words: string) => [...document.querySelectorAll<HTMLElement>(".armada-tooltip__label")].some((one) => one.textContent === words && one.checkVisibility({ visibilityProperty: true, opacityProperty: true }));

test("↵ from the title starts the Job, with the title he typed kept first and the landing in use", async () => {
  show(jobOn(startFor("feature")));
  await title().fill("Retry the client");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => begin.mock.calls.length).toBe(1);
  expect(begin).toHaveBeenCalledWith("s1", "auto_land");
  expect(rename).toHaveBeenCalledWith("s1", "Retry the client");
});

test("↑ and ↓ move between the rows, and ← and → change Kind and Lands", async () => {
  show(jobOn(startFor("feature")));
  await expect.element(page.getByRole("region", { name: "Start" })).toHaveFocus();
  await title().click();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(kind()).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(radio("Auto land")).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(model()).toHaveFocus();
  await userEvent.keyboard("{ArrowUp}{ArrowUp}");
  await expect.element(kind()).toHaveFocus();

  await userEvent.keyboard("{ArrowRight}");
  expect(choose).toHaveBeenCalledWith("s1", "bug");
  await expect.element(kind()).toHaveValue("bug");
  await userEvent.keyboard("{ArrowLeft}");
  expect(choose).toHaveBeenLastCalledWith("s1", "feature");

  await userEvent.keyboard("{ArrowDown}{ArrowRight}");
  await expect.element(radio("Open PR only")).toBeChecked();
  await expect.element(radio("Open PR only")).toHaveFocus();
  await userEvent.keyboard("{ArrowRight}{ArrowRight}");
  await expect.element(radio("Keep local")).toBeChecked();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(radio("Open PR only")).toBeChecked();
});

test("Esc in the title puts the edit back, and nothing is renamed", async () => {
  show(jobOn(startFor("feature")));
  await title().fill("Something else");
  await expect.element(title()).toHaveValue("Something else");
  await userEvent.keyboard("{Escape}");
  await expect.element(title()).toHaveValue("Retry the store client");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => begin.mock.calls.length).toBe(1);
  expect(rename).not.toHaveBeenCalled();
});

test("Start is off while no landing is picked where the kind asks, and ↵ goes to Lands instead of starting", async () => {
  show(jobOn({ asksDelivery: true }, { work: { proposal: { workflow: null, picked: [] } } }));
  await expect.element(startButton()).toBeDisabled();
  await title().click();
  await userEvent.keyboard("{Enter}");
  await expect.element(radio("Auto land")).toHaveFocus();
  expect(begin).not.toHaveBeenCalled();

  await radio("Auto land").click({ force: true });
  await expect.element(startButton()).toBeEnabled();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => begin.mock.calls.length).toBe(1);
  expect(begin).toHaveBeenCalledWith("s1", "auto_land");
});

test("the Lands row is not there where the kind has no landing step, and Start sends no landing", async () => {
  show(jobOn(startFor("review"), { work: { proposal: { workflow: "review", picked: [] } } }));
  await expect.element(kind()).toHaveValue("review");
  expect(document.querySelector('[role="radiogroup"]')).toBeNull();
  await expect.element(startButton()).toBeEnabled();
  await startButton().click();
  await expect.poll(() => begin.mock.calls.length).toBe(1);
  expect(begin).toHaveBeenCalledWith("s1", undefined);
});

test("Kind moves the row in and out of Lands as the kind asks", async () => {
  show(jobOn(startFor("feature")));
  await expect.element(page.getByRole("radiogroup", { name: "Lands" })).toBeVisible();
  await kind().selectOptions("review");
  await expect.poll(() => document.querySelector('[role="radiogroup"]')).toBeNull();
  await kind().selectOptions("bug");
  await expect.element(radio("Open PR only")).toBeChecked();
});

test("the usual mark says whose usual it is, and the named mark says the request named it", async () => {
  show(jobOn(startFor("feature")));
  await page.getByRole("img", { name: "Your usual for features in armada" }).hover();
  await expect.poll(() => bubble("Your usual for features in armada")).toBe(true);
  mounted?.root.unmount();
  mounted?.host.remove();

  show(jobOn(startFor("feature", "keep_local")));
  await expect.element(radio("Keep local")).toBeChecked();
  await page.getByRole("img", { name: "Named in your request" }).hover();
  await expect.poll(() => bubble("Named in your request")).toBe(true);
});

test("a landing he picked himself stays through a Kind change, and loses its mark; one he left alone moves with the kind", async () => {
  show(jobOn(startFor("feature")));
  await radio("Keep local").click({ force: true });
  await expect.element(radio("Keep local")).toBeChecked();
  expect(document.querySelector('[role="img"][aria-label^="Your usual"]')).toBeNull();
  await kind().selectOptions("bug");
  await expect.element(kind()).toHaveValue("bug");
  await expect.element(radio("Keep local")).toBeChecked();
  await startButton().click();
  await expect.poll(() => begin.mock.calls.length).toBe(1);
  expect(begin).toHaveBeenCalledWith("s1", "keep_local");

  mounted?.root.unmount();
  mounted?.host.remove();
  begin.mockClear();
  show(jobOn(startFor("feature")));
  await kind().selectOptions("bug");
  await expect.element(radio("Open PR only")).toBeChecked();
});

test("a model change goes to tune with the Job's mode sent back as it was, and the proposer's mark on it is let go", async () => {
  show(jobOn(startFor("feature"), { mode: "plan", work: { proposal: { workflow: "feature", picked: ["model"] } } }));
  await model().selectOptions("sonnet");
  expect(tune).toHaveBeenCalledWith("s1", { model: "sonnet", effort: "high", mode: "plan" });
  expect(unpick).toHaveBeenCalledWith("s1", "model");
});

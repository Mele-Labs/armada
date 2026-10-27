// The Workflow manager by itself, with every act a spy: what it groups, what a press hands its caller, what it says when the
// caller answers with Fleet's sentence, and the keys that reach it. The wiring to Fleet is `mock/workflows-wired.test.tsx`.

import { WorkflowManager, forgetAllKept } from "@armada/components";
import type { AgentState, Drift, ManagedStep, ManagedWorkflow, Suggestions, WorkflowManagerProps, WorkflowRef } from "@armada/components";
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  forgetAllKept();
});

const step = (id: string, label: string, words: string, over: Partial<ManagedStep> = {}): ManagedStep => ({ id, label, ends: `${label} ends`, words, ...over });

const ARMADA_STEPS = [step("propose", "Propose", "Say what you took it to mean."), step("walk", "Walk", "Build the mock.", { passedOver: "nothing would be seen" })];
const COPY_STEPS = [step("propose", "Propose", "Say what you took it to mean."), step("walk", "Walk", "Build the mock. Write no tests yet.")];

const WORKFLOWS: readonly ManagedWorkflow[] = [
  { id: "feature", source: "armada", forRequests: "adding something", steps: ARMADA_STEPS },
  { id: "bug", source: "armada", forRequests: "something broke", steps: [step("fix", "Fix", "Fix it.")] },
  { id: "feature", source: "repository", forRequests: "adding something", steps: COPY_STEPS },
  { id: "docs_only", source: "yours", forRequests: "words only", steps: [step("draft", "Draft", "Write it.")] },
  { id: "hotfix", source: "yours", forRequests: "", steps: [], leftOut: { said: "Names a check this repository does not declare", file: "~/.armada/workflows/hotfix.json" } },
];

const DRIFT: Drift = {
  feature: { walk: [{ id: "7", jobId: "job-7", job: "Cockpit timeline rail", what: "a test before the walk was approved", when: "2h ago" }] },
};

type Acts = {
  [K in keyof WorkflowManagerProps as WorkflowManagerProps[K] extends (...args: never[]) => unknown ? K : never]: ReturnType<typeof vi.fn>;
};

/** The manager over fixed props, every act a spy that goes through; `over` replaces props and spies. */
function draw(over: Partial<WorkflowManagerProps> = {}, first: WorkflowRef = { id: "feature", source: "repository" }): Acts {
  const done = async () => undefined;
  const acts = {
    onMakeCopy: vi.fn(done),
    onPropose: vi.fn(async () => undefined),
    onSaveWords: vi.fn(done),
    onReset: vi.fn(done),
    onDelete: vi.fn(done),
    onAsk: vi.fn(done),
    onApprove: vi.fn(done),
    onDismiss: vi.fn(done),
    onSendBack: vi.fn(done),
    onOpenJob: vi.fn(),
    onNotDrift: vi.fn(done),
    onSuggest: vi.fn(done),
    onApproveSuggestion: vi.fn(done),
    onDismissSuggestion: vi.fn(done),
    onPick: vi.fn(),
  };
  function Page() {
    const [picked, setPicked] = useState<WorkflowRef | null>(first);
    return (
      <WorkflowManager
        repository="armada"
        workflows={WORKFLOWS}
        drift={DRIFT}
        picked={picked}
        agent={{ phase: "idle" }}
        suggestions={{}}
        suggesting={null}
        {...acts}
        {...over}
        onPick={(one) => {
          acts.onPick(one);
          setPicked({ id: one.id, source: one.source });
        }}
      />
    );
  }
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  act(() => root.render(<Page />));
  return acts as unknown as Acts;
}

const step_ = (name: string) => page.getByRole("group", { name, exact: true });
const words = (name: string) => step_(name).getByRole("textbox", { name: "Words" });

test("workflows are grouped by where they come from, the repository's by its name, a copy marked with what it replaces", async () => {
  draw();
  for (const name of ["Armada", "armada repository", "Yours"]) await expect.element(page.getByRole("group", { name, exact: true })).toBeVisible();
  await expect.element(page.getByRole("group", { name: "Armada", exact: true }).getByRole("button", { name: "bug", exact: true })).toBeVisible();
  await expect.element(page.getByRole("group", { name: "armada repository", exact: true }).getByRole("button", { name: "feature", exact: true })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "This repository's copy replaces Armada's" })).toBeVisible();
  // Armada's row has nothing to replace.
  expect(page.getByRole("img", { name: /replaces/ }).elements()).toHaveLength(1);
});

test("a file Fleet left out shows with its reason, and has no steps to edit", async () => {
  draw({}, { id: "hotfix", source: "yours", file: "~/.armada/workflows/hotfix.json" });
  await expect.element(page.getByRole("img", { name: "Left out" })).toBeVisible();
  await expect.element(page.getByRole("paragraph")).toHaveTextContent("Names a check this repository does not declare");
  await expect.element(page.getByRole("button", { name: "Delete" })).toBeVisible();
  expect(page.getByRole("textbox", { name: "Words" }).elements()).toHaveLength(0);
});

test("the find box narrows the list, and Esc clears it", async () => {
  draw();
  await userEvent.fill(page.getByRole("searchbox", { name: "Search workflows" }), "docs");
  await expect.element(page.getByRole("button", { name: "docs_only", exact: true })).toBeVisible();
  expect(page.getByRole("button", { name: "bug", exact: true }).elements()).toHaveLength(0);
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("button", { name: "bug", exact: true })).toBeVisible();
});

test("Armada's steps are read-only, and Make my copy and Propose hand the workflow to their callers", async () => {
  const acts = draw({}, { id: "feature", source: "armada" });
  expect(page.getByRole("textbox", { name: "Words" }).elements()).toHaveLength(0);
  await page.getByRole("button", { name: "Make my copy" }).click();
  expect(acts.onMakeCopy).toHaveBeenCalledWith({ id: "feature", source: "armada" });
  await page.getByRole("button", { name: "Propose to Armada" }).click();
  expect(acts.onPropose).toHaveBeenCalledWith({ id: "feature", source: "armada" });
  await expect.element(page.getByText("Sent. A Job started on the Armada repository")).toBeVisible();
});

test("a proposal shows the Job it started, with a way to open it", async () => {
  const acts = draw({ onPropose: vi.fn(async () => ({ jobId: "job-9" })) }, { id: "feature", source: "armada" });
  await page.getByRole("button", { name: "Propose to Armada" }).click();
  await page.getByRole("status").getByRole("button", { name: "Open Job" }).click();
  expect(acts.onOpenJob).toHaveBeenCalledWith({ id: "feature", source: "armada" }, "job-9");
});

test("words edited in place show Save and Discard; Save hands the words over and Discard puts them back", async () => {
  const acts = draw();
  await userEvent.fill(words("Propose"), "Say it twice.");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  expect(acts.onSaveWords).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "propose", "Say it twice.");
  await expect.element(page.getByRole("button", { name: "Save", exact: true })).not.toBeInTheDocument();

  await userEvent.fill(words("Walk"), "Something else.");
  await page.getByRole("button", { name: "Discard" }).click();
  await expect.element(words("Walk")).toHaveValue("Build the mock. Write no tests yet.");
  expect(acts.onSaveWords).toHaveBeenCalledTimes(1);
});

test("a save Fleet refused is said as Fleet said it, and the words typed stay", async () => {
  draw({ onSaveWords: vi.fn(async () => "Names a check this repository does not declare") });
  await userEvent.fill(words("Propose"), "Say it twice.");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect.element(page.getByRole("alert")).toHaveTextContent("Names a check this repository does not declare");
  await expect.element(words("Propose")).toHaveValue("Say it twice.");
  await expect.element(page.getByRole("button", { name: "Save", exact: true })).toBeVisible();
});

test("E in a focused step goes into its words", async () => {
  draw();
  step_("Walk").element().focus();
  await userEvent.keyboard("e");
  await expect.poll(() => document.activeElement).toBe(words("Walk").element());
});

test("F reaches the find box from a page with nothing focused", async () => {
  draw();
  (document.activeElement as HTMLElement | null)?.blur();
  await userEvent.keyboard("f");
  await expect.poll(() => document.activeElement).toBe(page.getByRole("searchbox", { name: "Search workflows" }).element());
});

test("Compare with Armada's marks the step that differs and sets its words beside Armada's", async () => {
  draw();
  expect(page.getByRole("group", { name: "Compared with Armada's" }).elements()).toHaveLength(0);
  await page.getByRole("button", { name: "Compare with Armada's" }).click();
  await expect.element(step_("Walk").getByRole("group", { name: "Compared with Armada's" })).toBeVisible();
  await expect.element(step_("Walk").getByRole("img", { name: "Differs from Armada's" })).toBeVisible();
  expect(step_("Propose").getByRole("img").elements()).toHaveLength(0);
});

test("Reset to Armada's hands the copy over and says it matches", async () => {
  const acts = draw();
  await page.getByRole("button", { name: "Reset to Armada's" }).click();
  expect(acts.onReset).toHaveBeenCalledWith({ id: "feature", source: "repository" });
  await expect.element(page.getByText("Matches Armada's")).toBeVisible();
});

test("a reset Fleet refused says why and does not say it matches", async () => {
  draw({ onReset: vi.fn(async () => "Armada's does not fit here") });
  await page.getByRole("button", { name: "Reset to Armada's" }).click();
  await expect.element(page.getByRole("alert")).toHaveTextContent("Armada's does not fit here");
  expect(page.getByText("Matches Armada's").elements()).toHaveLength(0);
});

test("Delete hands the copy to its caller", async () => {
  const acts = draw({}, { id: "docs_only", source: "yours" });
  await page.getByRole("button", { name: "Delete" }).click();
  expect(acts.onDelete).toHaveBeenCalledWith({ id: "docs_only", source: "yours" });
});

test("a Job Fleet nudged opens from its row, or by O, and Not a drift takes it by the button or by N", async () => {
  const acts = draw();
  await page.getByRole("button", { name: "Open Job" }).click();
  expect(acts.onOpenJob).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "job-7");

  const row = page.getByRole("group", { name: "Cockpit timeline rail, a test before the walk was approved" });
  row.element().focus();
  await userEvent.keyboard("o");
  expect(acts.onOpenJob).toHaveBeenCalledTimes(2);
  await userEvent.keyboard("n");
  expect(acts.onNotDrift).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "walk", "7");
  await page.getByRole("button", { name: "Not a drift" }).click();
  expect(acts.onNotDrift).toHaveBeenCalledTimes(2);
});

const SUGGESTED: Suggestions = { feature: { walk: { words: "Build the mock. Write no tests yet. Wait for the walk.", from: ["7"] } } };

test("a suggested change shows before and after; Approve and Dismiss hand it back, and so do ⌘Enter and Esc", async () => {
  const acts = draw({ suggestions: SUGGESTED });
  const card = step_("Walk").getByRole("group", { name: "Suggested change" });
  await expect.element(card.getByRole("group", { name: "Before and after" })).toBeVisible();
  await expect.element(step_("Walk").getByRole("img", { name: "In the suggested change" })).toBeVisible();

  await card.getByRole("button", { name: "Approve", exact: true }).click();
  expect(acts.onApproveSuggestion).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "walk");
  await card.getByRole("button", { name: "Dismiss" }).click();
  expect(acts.onDismissSuggestion).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "walk");

  card.element().focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(acts.onApproveSuggestion).toHaveBeenCalledTimes(2);
  // A suggestion taken sends focus back to its step; Esc is the card's own while focus is in it.
  card.element().focus();
  await userEvent.keyboard("{Escape}");
  expect(acts.onDismissSuggestion).toHaveBeenCalledTimes(2);
});

test("an Approve Fleet refused on a suggestion is said on the card", async () => {
  draw({ suggestions: SUGGESTED, onApproveSuggestion: vi.fn(async () => "The file changed since the draft was made") });
  const card = step_("Walk").getByRole("group", { name: "Suggested change" });
  await card.getByRole("button", { name: "Approve", exact: true }).click();
  await expect.element(card.getByRole("alert")).toHaveTextContent("The file changed since the draft was made");
});

test("a step with drift and no suggestion offers Suggest a change, and shows the agent working while one is drawn", async () => {
  const acts = draw();
  await page.getByRole("button", { name: "Suggest a change" }).click();
  expect(acts.onSuggest).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "walk");

  draw({ suggesting: "feature/walk" });
  await expect.element(page.getByRole("img", { name: "The agent is working" })).toBeVisible();
});

test("a draft that did not fit is said with Fleet's reason, and can be dismissed", async () => {
  const acts = draw({ suggestions: { feature: { walk: { words: "", from: [], failed: "The drafted workflow is the same as the one it was made from" } } } });
  const card = step_("Walk").getByRole("group", { name: "Suggested change" });
  await expect.element(card).toHaveTextContent("The drafted workflow is the same as the one it was made from");
  expect(card.getByRole("button", { name: "Approve", exact: true }).elements()).toHaveLength(0);
  await card.getByRole("button", { name: "Dismiss" }).click();
  expect(acts.onDismissSuggestion).toHaveBeenCalledOnce();
});

test("the agent box sends what was typed by the button or ⌘Enter, and keeps it when Fleet refuses", async () => {
  const acts = draw();
  const box = page.getByRole("textbox", { name: "Tell an agent what to change" });
  await expect.element(page.getByRole("button", { name: "Send to agent" })).toBeDisabled();
  await userEvent.fill(box, "split Refine into Wire and Tests");
  await page.getByRole("button", { name: "Send to agent" }).click();
  expect(acts.onAsk).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "split Refine into Wire and Tests");
  await expect.element(box).toHaveValue("");

  await userEvent.fill(box, "again");
  box.element().focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(acts.onAsk).toHaveBeenCalledTimes(2);
});

test("a request Fleet refused is said and stays typed", async () => {
  draw({ onAsk: vi.fn(async () => "No model could be asked") });
  const box = page.getByRole("textbox", { name: "Tell an agent what to change" });
  await userEvent.fill(box, "add a step");
  await page.getByRole("button", { name: "Send to agent" }).click();
  await expect.element(page.getByRole("alert")).toHaveTextContent("No model could be asked");
  await expect.element(box).toHaveValue("add a step");
});

const BEFORE = [step("propose", "Propose", "a"), step("refine", "Refine", "b")];
const AFTER = [step("propose", "Propose", "a"), step("wire", "Wire", "c"), step("tests", "Tests", "d")];
const PROPOSED: AgentState = { phase: "proposed", before: BEFORE, after: AFTER, asked: "split Refine into Wire and Tests" };

test("a drafting change shows what was asked and the agent working", async () => {
  draw({ agent: { phase: "working", asked: "split Refine into Wire and Tests" } });
  await expect.element(page.getByRole("img", { name: "The agent is working" })).toBeVisible();
  await expect.element(page.getByText("split Refine into Wire and Tests")).toBeVisible();
  expect(page.getByRole("textbox", { name: "Tell an agent what to change" }).elements()).toHaveLength(0);
});

test("a ready change shows before and after, with Approve, Send back with notes, and Dismiss", async () => {
  const acts = draw({ agent: PROPOSED });
  const region = page.getByRole("region", { name: "Proposed change" });
  await expect.element(region.getByRole("group", { name: "Before" }).getByText("Refine", { exact: true })).toBeVisible();
  await expect.element(region.getByRole("group", { name: "After" }).getByText("Wire", { exact: true })).toBeVisible();
  await expect.element(region.getByRole("img", { name: "Removed" })).toBeVisible();
  expect(region.getByRole("img", { name: "Added" }).elements()).toHaveLength(2);

  await region.getByRole("button", { name: "Approve", exact: true }).click();
  expect(acts.onApprove).toHaveBeenCalledWith({ id: "feature", source: "repository" });
  await region.getByRole("button", { name: "Dismiss" }).click();
  expect(acts.onDismiss).toHaveBeenCalledWith({ id: "feature", source: "repository" });

  await region.getByRole("button", { name: "Send back" }).click();
  await expect.element(region.getByRole("button", { name: "Send notes" })).toBeDisabled();
  const notes = region.getByRole("textbox", { name: "Notes for the agent" });
  await userEvent.fill(notes, "Keep Refine");
  await region.getByRole("button", { name: "Send notes" }).click();
  expect(acts.onSendBack).toHaveBeenCalledWith({ id: "feature", source: "repository" }, "Keep Refine");

  await userEvent.fill(region.getByRole("textbox", { name: "Notes for the agent" }), "Keep it, again");
  region.getByRole("textbox", { name: "Notes for the agent" }).element().focus();
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  expect(acts.onSendBack).toHaveBeenLastCalledWith({ id: "feature", source: "repository" }, "Keep it, again");
});

test("Esc in the notes closes them and leaves the change", async () => {
  draw({ agent: PROPOSED });
  const region = page.getByRole("region", { name: "Proposed change" });
  await region.getByRole("button", { name: "Send back" }).click();
  region.getByRole("textbox", { name: "Notes for the agent" }).element().focus();
  await userEvent.keyboard("{Escape}");
  expect(region.getByRole("textbox", { name: "Notes for the agent" }).elements()).toHaveLength(0);
  await expect.element(region).toBeVisible();
});

test("an Approve Fleet refused, because the file changed, is drawn as Fleet said it", async () => {
  draw({ agent: PROPOSED, onApprove: vi.fn(async () => "feature.json changed since the draft was made. Dismiss the draft and ask again") });
  const region = page.getByRole("region", { name: "Proposed change" });
  await region.getByRole("button", { name: "Approve", exact: true }).click();
  await expect.element(region.getByRole("alert")).toHaveTextContent("feature.json changed since the draft was made. Dismiss the draft and ask again");
  await expect.element(region).toBeVisible();
});

test("a draft that failed says why and can be dismissed", async () => {
  const acts = draw({ agent: { phase: "failed", reason: "The checks refuse it: names a step twice" } });
  await expect.element(page.getByRole("alert")).toHaveTextContent("The checks refuse it: names a step twice");
  await page.getByRole("button", { name: "Dismiss" }).click();
  expect(acts.onDismiss).toHaveBeenCalledOnce();
});

test("a step with no glyph of its own draws none, and one Fleet gives no end for says no end", async () => {
  draw({ workflows: [{ id: "custom", source: "yours", forRequests: "", steps: [step("odd", "Odd", "Words.", { ends: "" })] }] }, { id: "custom", source: "yours" });
  await expect.element(step_("Odd")).toBeVisible();
  expect(step_("Odd").getByRole("term").elements().map((one) => one.textContent)).toEqual(["Words"]);
});

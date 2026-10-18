// What a Session's done callout lists, and the marks a pull request row carries there and in the ledger,
// through `App`. The walks `session-all-done` and `session-all-done-artifacts` draw them; this holds
// the order, the marks and the left-out rows. The spin itself is not asserted: the browser project runs
// with reduced motion, so a running mark is held by its glyph and its `data-checks`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "./scenario";
import { s200Sessions } from "./scenarios/sessions";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const pr = (number: number, state: "open" | "merged", checks: Extract<SessionAttachment, { kind: "pull_request" }>["checks"]): SessionAttachment => ({
  kind: "pull_request",
  number,
  title: `Change ${number}`,
  branch: `fix/${number}`,
  address: `https://example.com/pull/${number}`,
  checks,
  state,
  auto: false,
});

const artifact = (form: "page" | "file" | "doc" | "image" | "window", title: string): SessionAttachment => ({ kind: "artifact", form, id: `https://example.com/${title}`, title });

/** A finished Session titled `Callout subject` over the `sessions` scenario, holding `attachments`. */
function holding(attachments: SessionAttachment[], waitingFor: Session["waitingFor"] = undefined): Scenario {
  const now = new Date().toISOString();
  const session: Session = {
    id: "s14",
    title: "Callout subject",
    turn: { state: "idle" },
    lastTurn: "14:40",
    lastTurnAt: now,
    rows: [{ id: "d1", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "Done." }],
    attachments,
    done: { since: now },
    ...(waitingFor === undefined ? {} : { waitingFor }),
  };
  return { ...s200Sessions, name: "session-all-done-test", draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [session]) } };
}

/** The Session on screen, with the end band drawn and folded. */
async function show(scenario: Scenario) {
  mount(scenario);
  await onScreen();
  await toSession(/Callout subject/);
  const band = page.getByRole("region", { name: "Nothing left to do" });
  await expect.element(band.getByRole("button", { name: "Nothing left to do" })).toBeVisible();
  return band;
}

/** The same, with the band pressed open so the list of what was made is there to read. */
async function open(scenario: Scenario) {
  const band = await show(scenario);
  await userEvent.click(band.getByRole("button", { name: "Nothing left to do" }));
  return band;
}

const names = (rows: Element[]) => rows.map((one) => one.getAttribute("aria-label"));

test("the callout lists merged pull requests first and then open ones, each with its state and checks marks", async () => {
  const callout = await open(holding([pr(3, "open", { state: "pending" }), pr(1, "merged", { state: "passed" }), pr(2, "merged", { state: "failed", failing: "lint" })]));
  const list = callout.getByRole("list", { name: "Pull requests" });
  const rows = list.getByRole("listitem");
  await expect.element(rows.nth(2)).toBeVisible();
  expect(names(rows.elements())).toEqual(["Pull request #1, checks passed", "Pull request #2, checks failed", "Pull request #3, checks pending"]);

  // Every mark is drawn without a hover. A mark's tooltip and its name are one string, so the name is what is held.
  await expect.element(rows.nth(0).getByRole("img", { name: "Merged" })).toBeVisible();
  await expect.element(rows.nth(0).getByRole("img", { name: "Checks passed" })).toBeVisible();
  await expect.element(rows.nth(1).getByRole("img", { name: "Merged" })).toBeVisible();
  await expect.element(rows.nth(1).getByRole("img", { name: "Checks failed: lint" })).toBeVisible();
  await expect.element(rows.nth(2).getByRole("img", { name: "Open" })).toBeVisible();
  const running = rows.nth(2).getByRole("img", { name: "Checks running" });
  await expect.element(running).toBeVisible();
  expect(running.element().getAttribute("data-checks")).toBe("running");
  expect(running.element().querySelector("svg")?.classList.contains("lucide-loader-circle")).toBe(true);
});

test("with no pull request the callout lists the pages, files and Docs, and leaves out pictures and windows", async () => {
  const callout = await open(holding([artifact("page", "Findings"), artifact("image", "shot.png"), artifact("file", "notes.md"), artifact("window", "Preview"), artifact("doc", "Write-up")]));
  const list = callout.getByRole("list", { name: "Artifacts" });
  await expect.element(list).toBeVisible();
  expect(names(list.getByRole("listitem").elements())).toEqual(["Published page Findings", "File written notes.md", "Doc Write-up"]);
  await expect.element(callout.getByRole("list", { name: "Pull requests" })).not.toBeInTheDocument();
});

test("pull requests win over artifacts", async () => {
  const callout = await open(holding([artifact("page", "Findings"), pr(1, "merged", { state: "passed" })]));
  await expect.element(callout.getByRole("list", { name: "Pull requests" })).toBeVisible();
  await expect.element(callout.getByRole("list", { name: "Artifacts" })).not.toBeInTheDocument();
});

test("with neither the callout has no list at all", async () => {
  const callout = await open(holding([artifact("image", "shot.png")]));
  await expect.element(callout.getByRole("button", { name: "End job" })).toBeVisible();
  expect(callout.element().querySelector("ul")).toBeNull();
});

test("the ledger's pull request rows show their state and checks marks without a hover", async () => {
  // A merged row is finished, and the ledger draws it under All only.
  localStorage.removeItem("armada.session-ledger.show.pull_request");
  await show(holding([pr(1, "merged", { state: "passed" }), pr(2, "open", { state: "pending" }), pr(3, "open", { state: "failed", failing: "lint" })]));
  const section = page.getByRole("region", { name: "Attachments" }).getByRole("region", { name: "Pull requests" });
  const rows = section.getByRole("listitem");
  await expect.element(rows.nth(1)).toBeVisible();
  expect(names(rows.elements())).toEqual(["Pull request #2, checks pending", "Pull request #3, checks failed"]);

  await expect.element(rows.nth(0).getByRole("img", { name: "Open" })).toBeVisible();
  const running = rows.nth(0).getByRole("img", { name: "Checks running" });
  await expect.element(running).toBeVisible();
  expect(running.element().getAttribute("data-checks")).toBe("running");
  expect(running.element().querySelector("svg")?.classList.contains("lucide-loader-circle")).toBe(true);
  await expect.element(rows.nth(1).getByRole("img", { name: "Checks failed: lint" })).toBeVisible();

  await userEvent.click(section.getByRole("radio", { name: "All" }));
  await expect.element(rows.nth(2)).toBeVisible();
  await expect.element(rows.nth(0).getByRole("img", { name: "Merged" })).toBeVisible();
  await expect.element(rows.nth(0).getByRole("img", { name: "Checks passed" })).toBeVisible();
  localStorage.removeItem("armada.session-ledger.show.pull_request");
});

test("a done Session with something waiting keeps it in the Needs you band, draws no end band, and the ledger draws no Waiting on you section", async () => {
  const since = new Date().toISOString();
  mount(holding([pr(5, "open", { state: "passed" })], [{ id: "w1", text: "Approve #5", since, source: "agent", act: { kind: "approve_pr", target: "5" } }]));
  await onScreen();
  await toSession(/Callout subject/);
  const band = page.getByRole("region", { name: "Needs you" });
  await expect.element(band).toBeVisible();
  await userEvent.click(band.getByRole("button", { name: /Needs you/ }));
  await expect.element(band.getByRole("button", { name: "Open Approve #5" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Nothing left to do" })).not.toBeInTheDocument();
  const ledger = page.getByRole("region", { name: "Attachments" });
  await expect.element(ledger.getByRole("region", { name: "Pull requests" })).toBeVisible();
  await expect.element(ledger.getByRole("region", { name: "Waiting on you" })).not.toBeInTheDocument();
  await expect.element(ledger.getByRole("button", { name: "Open Approve #5" })).not.toBeInTheDocument();
});

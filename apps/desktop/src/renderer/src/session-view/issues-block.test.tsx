// Progress's issues: each with the pull requests that close it, the pull requests that close none, and the presses that open them in the side
// panel. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { openInPanel } from "../shared-panel/store";
import { IssuesBlock, issuesOf, issuesOutsidePlan } from "./IssuesBlock";

vi.mock("../shared-panel/store", () => ({ openInPanel: vi.fn(() => true) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  vi.mocked(openInPanel).mockClear();
});

const issue = (number: number, state: "open" | "closed" = "open"): SessionAttachment => ({ kind: "issue", number: String(number), title: `Issue ${number}`, address: `https://x/issues/${number}`, state });
const pr = (number: number, closes?: number[]): SessionAttachment => ({
  kind: "pull_request",
  number,
  title: `Pull ${number}`,
  branch: `b${number}`,
  address: `https://x/pull/${number}`,
  checks: { state: "passed" },
  state: "open",
  auto: false,
  ...(closes === undefined ? {} : { closes }),
});
const sessionOf = (attachments: SessionAttachment[], groups: { id: string; issue?: string }[] = []) =>
  ({
    id: "s1",
    attachments,
    rows: [],
    turn: { state: "idle" },
    work: groups.length === 0 ? undefined : { plan: { scope: [], approved: false, groups: groups.map((g) => ({ ...g, title: g.id, agent: "a", paths: [], tasks: [] })) } },
  }) as unknown as Session;

const numbers = (rows: { number: number | string }[]) => rows.map((one) => one.number);

test("an issue is listed with the pull requests that close it, and loose pull requests come last", () => {
  const { issues, loose } = issuesOf(sessionOf([pr(30), issue(1), pr(10, [1]), issue(2), pr(20, [2])]));
  expect(issues.map((one) => [one.issue.number, numbers(one.prs)])).toEqual([
    ["1", [10]],
    ["2", [20]],
  ]);
  expect(numbers(loose)).toEqual([30]);
});

test("a pull request that closes two of the issues stands under both and is not loose", () => {
  const { issues, loose } = issuesOf(sessionOf([issue(1), issue(2), pr(10, [1, 2])]));
  expect(issues.map((one) => numbers(one.prs))).toEqual([[10], [10]]);
  expect(loose).toEqual([]);
});

test("a pull request that closes an issue the Session does not hold is loose", () => {
  expect(numbers(issuesOf(sessionOf([issue(1), pr(10, [99])])).loose)).toEqual([10]);
});

test("an issue a plan group names leaves the block, and the pull requests that close none stay", () => {
  const session = sessionOf([issue(1), issue(2), pr(10, [1]), pr(30)], [{ id: "g1", issue: "1" }, { id: "g2" }]);
  const { issues, loose } = issuesOutsidePlan(session);
  expect(issues.map((one) => one.issue.number)).toEqual(["2"]);
  expect(numbers(loose)).toEqual([30]);
});

function mount(session: Session): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<IssuesBlock session={session} manifestId="armada" />));
  roots.push({ root, host });
  return host;
}

test("with no issue and no pull request nothing is drawn", () => {
  expect(mount(sessionOf([])).innerHTML).toBe("");
});

test("pressing an issue opens it in the side panel, with its title and the Manifest", () => {
  const host = mount(sessionOf([issue(2212), pr(2256, [2212])]));
  act(() => host.querySelector<HTMLElement>('[aria-label="Issue #2212 Issue 2212"]')!.click());
  expect(openInPanel).toHaveBeenCalledExactlyOnceWith({ kind: "issue", number: "2212", address: "https://x/issues/2212", title: "Issue 2212", manifestId: "armada" });
});

test("a tracker's issue holds the pull requests its row names, drawn by its key, and a numbered one keeps its #", () => {
  const keyed: SessionAttachment = { kind: "issue", number: "ABC-12", title: "Checkout total rounds twice", address: "https://tracker.invalid/browse/ABC-12", state: "open", pull_requests: [31] };
  const session = sessionOf([keyed, pr(31), issue(7), pr(10, [7])]);
  const { issues, loose } = issuesOf(session);
  expect(issues.map((one) => [one.issue.number, numbers(one.prs)])).toEqual([
    ["ABC-12", [31]],
    ["7", [10]],
  ]);
  expect(loose).toEqual([]);
  const host = mount(session);
  expect([...host.querySelectorAll(".sv-strip__num")].map((one) => one.textContent)).toEqual(["ABC-12", "#7"]);
  expect(host.querySelector('[aria-label="Issue ABC-12 Checkout total rounds twice"]')).not.toBeNull();
  expect(host.querySelector('[aria-label="Pull requests for ABC-12"] [aria-label="Pull request #31 Pull 31"]')).not.toBeNull();
  expect(host.querySelector("[data-loose]")).toBeNull();
});

test("pressing a pull request opens it in the side panel", () => {
  const host = mount(sessionOf([issue(2212), pr(2256, [2212])]));
  act(() => host.querySelector<HTMLElement>('[aria-label="Pull request #2256 Pull 2256"]')!.click());
  expect(openInPanel).toHaveBeenCalledExactlyOnceWith({ kind: "pr", number: 2256, address: "https://x/pull/2256" });
});

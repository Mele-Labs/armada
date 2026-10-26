// The Landing in the Job's right column: what the workflow ends in, so it differs by kind. A pull request draws the pull request list, a spike its findings, a mod the
// mods it made; the issues the Job filed for later stand in Deferred. Browser: the drawing is what is read.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import type { ModSummary } from "@armada/protocol";
import type { Session, SessionAttachment, WorkLanding } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../shared/bridge";
import { NOTHING_YET } from "../../../shared/bridge";
import { LandingPanel } from "./LandingPanel";
import { resetFolds } from "./story";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ID = "landing-s1";
const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  resetFolds(ID);
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  Object.defineProperty(window, "armada", { value: undefined, configurable: true });
});

const pull = (number: number, state: "draft" | "open" | "merged" = "open"): SessionAttachment => ({ kind: "pull_request", number, title: `Pull ${number}`, branch: `armada/b${number}`, address: `https://forge/pull/${number}`, checks: { state: "passed" }, state, auto: false });
const issue = (number: number): SessionAttachment => ({ kind: "issue", number: String(number), title: `Issue ${number}`, address: `https://forge/issues/${number}`, state: "open" });
const doc = (id: string, title: string): SessionAttachment => ({ kind: "artifact", form: "doc", id, title });

function sessionOf(landing: WorkLanding | undefined, attachments: SessionAttachment[], extra: Partial<Session> = {}): Session {
  return { id: ID, attachments, rows: [], turn: { state: "idle" }, ...(landing === undefined ? {} : { work: { landing } }), ...extra } as unknown as Session;
}

async function mount(session: Session): Promise<HTMLElement> {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<LandingPanel session={session} entries={[]} place="side" id={session.id} />));
  roots.push({ root, host });
  return host;
}

const titles = (host: HTMLElement, list: string) => [...host.querySelectorAll(`[aria-label="${list}"] > li`)].map((one) => one.querySelector("button")?.getAttribute("aria-label"));

test("a pull request landing draws the pull requests as rows, the newest first", async () => {
  const host = await mount(sessionOf("pull_request", [pull(7), pull(8)]));
  expect(titles(host, "Pull requests")).toEqual(["Pull request #8 Pull 8", "Pull request #7 Pull 7"]);
  expect(host.querySelector('[aria-label="Findings"]')).toBeNull();
});

test("a Job with no landing named lands in a pull request", async () => {
  const host = await mount(sessionOf(undefined, [pull(7)]));
  expect(titles(host, "Pull requests")).toEqual(["Pull request #7 Pull 7"]);
});

test("a pull request row names its branch and how its Checks stand", async () => {
  const host = await mount(sessionOf("pull_request", [pull(7)]));
  const row = host.querySelector('[aria-label="Pull requests"] > li')!;
  expect(row.textContent).toContain("armada/b7");
  expect(row.textContent).toContain("Passed");
});

test("a merged pull request settles the band as landed", async () => {
  const host = await mount(sessionOf("pull_request", [pull(7, "merged")]));
  expect(host.querySelector("section.sv-panel")?.getAttribute("data-state")).toBe("done");
});

test("a spike landing draws the doc it made as Findings, and no pull request list", async () => {
  const host = await mount(sessionOf("spike", [doc("docs/findings.md", "Why the lease drops"), pull(7)]));
  expect(titles(host, "Findings")).toEqual(["Why the lease drops"]);
  expect(host.querySelector('[aria-label="Pull requests"]')).toBeNull();
});

test("a spike that has written nothing yet draws no Findings", async () => {
  const host = await mount(sessionOf("spike", []));
  expect(host.querySelector('[aria-label="Findings"]')).toBeNull();
});

function fleet(mods: ModSummary[]) {
  const api = {
    syncState: async () => ({ full: { ...NOTHING_YET, mods: { mods } } satisfies BridgeState, seq: 1 }),
    subscribeChanges: () => () => undefined,
    setModEnabled: vi.fn(),
  };
  Object.defineProperty(window, "armada", { value: api, configurable: true });
}
const mod = (name: string, extra: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "theme", enabled: true, valid: true, ...extra });

test("a mod landing draws the mods this Job made, each with its switch, and none made by another", async () => {
  fleet([mod("dusk", { made_by: ID }), mod("by-another", { made_by: "other" })]);
  const host = await mount(sessionOf("mod", []));
  expect([...host.querySelectorAll('[role="switch"]')].map((one) => one.getAttribute("aria-label"))).toEqual(["Dusk"]);
  expect(host.querySelector('[aria-label="Pull requests"]')).toBeNull();
});

test("a landing in a pull request draws no mods", async () => {
  fleet([mod("dusk", { made_by: ID })]);
  const host = await mount(sessionOf("pull_request", [pull(7)]));
  expect(host.querySelector('[role="switch"]')).toBeNull();
});

// ---- Deferred ------------------------------------------------------------------------------------------------

const deferred = (host: HTMLElement) => host.querySelector<HTMLElement>('section[aria-label="Deferred"]');

test("Deferred lists the issues the Job filed, once it is opened", async () => {
  const host = await mount(sessionOf("pull_request", [pull(7), issue(31), issue(32)]));
  const band = deferred(host)!.querySelector<HTMLElement>("button.sv-later__row")!;
  expect(band.getAttribute("aria-expanded")).toBe("false");
  expect(deferred(host)!.querySelector(".sv-later__list")).toBeNull();
  act(() => band.click());
  expect([...deferred(host)!.querySelectorAll('.sv-later__item[data-kind="issue"] button')].map((one) => one.getAttribute("aria-label"))).toEqual(["Issue #31 Issue 31", "Issue #32 Issue 32"]);
});

test("Deferred keeps the follow-ups the agent set aside beside the issues, and offers Remind and Drop on the follow-ups alone", async () => {
  const host = await mount(sessionOf("pull_request", [pull(7), issue(31)], { later: [{ id: "l1", text: "Check the retry on Windows" }] } as Partial<Session>));
  act(() => deferred(host)!.querySelector<HTMLElement>("button.sv-later__row")!.click());
  const items = [...deferred(host)!.querySelectorAll(".sv-later__item")];
  expect(items).toHaveLength(2);
  expect(deferred(host)!.querySelector('[aria-label="Remind: Check the retry on Windows"]')).not.toBeNull();
  expect(items[1]!.querySelector('[aria-label^="Remind"]')).toBeNull();
});

test("an issue a plan group works stands in the Blueprint, not in Deferred", async () => {
  const plan = { scope: [], approved: true, groups: [{ id: "g1", title: "Retry", agent: "a", paths: [], issue: "31", tasks: [] }] };
  const host = await mount(sessionOf("pull_request", [pull(7), issue(31)], { work: { landing: "pull_request", plan } } as Partial<Session>));
  expect(deferred(host)).toBeNull();
});

test("a Job with nothing filed and nothing set aside draws no Deferred", async () => {
  expect(deferred(await mount(sessionOf("pull_request", [pull(7)])))).toBeNull();
});

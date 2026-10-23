// What a thread's links hold across redraws: the same `ProseRefs` while nothing they can link has changed, since a new one redraws
// every message in the thread and reads each of them again. A press still goes to the latest `openSession`.

import { act, useContext } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { ProseRefs } from "@armada/components";
import type { ProseReferences } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { RefsHost, SessionRefs } from "./helm-refs";
import type { Host } from "./helm-refs";
import { SessionsFrom } from "./sessions-draft";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];

/** Fleet's state as a window holds it before any mod exists: the thread reads the mod list, so a window has one. */
const QUIET = { syncState: async () => ({ full: { mods: null }, seq: 1 }), subscribeChanges: () => undefined };
beforeEach(() => {
  (window as unknown as { armada: unknown }).armada = { ...QUIET, findPanelFiles: async () => [] };
});
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const session = (id: string, title: string, lastTurnAt = "2026-10-10T09:00:00Z"): Session =>
  ({ id, title, manifestId: "armada", turn: { state: "idle" }, lastTurnAt, rows: [], attachments: [] }) as unknown as Session;

/** Sessions that change when told to, as a real Fleet's list does with any Session's message. */
function store(initial: Session[]) {
  let now: readonly Session[] = initial;
  const listeners = new Set<() => void>();
  const held = { get: () => now, subscribe: (listener: () => void) => (listeners.add(listener), () => listeners.delete(listener)) } as unknown as SessionsDraft;
  return {
    held,
    set: (next: Session[]) =>
      act(() => {
        now = next;
        listeners.forEach((one) => one());
      }),
  };
}

/** A host that is new each time, as `App` builds it on every render. */
const hostOf = (openSession: (id: string) => void): Host => ({ state: {} as BridgeState, repositories: [], openSession, openLink: () => undefined });

const THREAD = session("01J9ZK7M3Q8T5V2X4Y6Z0A1B2C", "Retry tuning");
const OTHER = session("01J9ZM1P3Q8T5V2X4Y6Z0A1B2D", "Lint sweep");

function draw(sessions: Session[], thread: Session = THREAD) {
  const seen: (ProseReferences | null)[] = [];
  function Probe() {
    seen.push(useContext(ProseRefs));
    return null;
  }
  const mount = document.body.appendChild(document.createElement("div"));
  const root = createRoot(mount);
  roots.push({ root, host: mount });
  const held = store(sessions);
  const render = (host: Host) =>
    act(() =>
      root.render(
        <RefsHost.Provider value={host}>
          <SessionsFrom held={held.held}>
            <SessionRefs session={thread}>
              <Probe />
            </SessionRefs>
          </SessionsFrom>
        </RefsHost.Provider>,
      ),
    );
  return { seen, render, set: held.set };
}

test("the links stay the same object when the host, its openers or another Session's unrelated field change", () => {
  const { seen, render, set } = draw([THREAD, OTHER]);
  render(hostOf(() => undefined));
  render(hostOf(() => undefined));
  set([THREAD, { ...OTHER, lastTurnAt: "2026-10-10T10:00:00Z" }]);
  // A value that did not change is not drawn again, so the store's change shows as no new entry, never as another object.
  expect(seen.length).toBeGreaterThanOrEqual(2);
  expect(seen[0]).not.toBeNull();
  expect(new Set(seen).size).toBe(1);
});

test("the links are new where a Session's title is, since that is a word they link", () => {
  const { seen, render, set } = draw([THREAD, OTHER]);
  render(hostOf(() => undefined));
  const before = seen.at(-1);
  set([THREAD, { ...OTHER, title: "Lint the crates" }]);
  const after = seen.at(-1)!;
  expect(after).not.toBe(before);
  expect(after.scan("Lint the crates").map((one) => one.ref.id)).toEqual([OTHER.id]);
  expect(after.scan("Lint sweep")).toEqual([]);
});

test("pressing a link opens the Session with the latest `openSession`", () => {
  const { seen, render } = draw([THREAD, OTHER]);
  const first = vi.fn();
  const latest = vi.fn();
  render(hostOf(first));
  render(hostOf(latest));
  seen.at(-1)!.open({ kind: "session", id: OTHER.id, title: "Lint sweep" });
  expect(first).not.toHaveBeenCalled();
  expect(latest).toHaveBeenCalledWith(OTHER.id);
});

test("a path in a code span links once main says it is a file in the Job's checkout, and is asked of main once", async () => {
  const find = vi.fn(async (_id: string, paths: readonly string[]) => paths.filter((one) => one === "docs/plan.md"));
  (window as unknown as { armada: unknown }).armada = { ...QUIET, findPanelFiles: find };
  const said = { ...THREAD, rows: [{ kind: "message", id: "m1", from: "agent", at: "", text: "Read `docs/plan.md` and `docs/gone.md`" }] } as unknown as Session;
  const { seen, render } = draw([said, OTHER], said);
  await act(async () => render(hostOf(() => undefined)));
  await act(async () => render(hostOf(() => undefined)));
  const refs = seen.at(-1)!;
  expect(refs.scan("docs/plan.md").map((one) => [one.ref.kind, one.ref.id])).toEqual([["file", "docs/plan.md"]]);
  expect(refs.scan("docs/gone.md")).toEqual([]);
  expect(find).toHaveBeenCalledTimes(1);
  expect(find).toHaveBeenCalledWith(said.id, ["docs/plan.md", "docs/gone.md"]);
});

/** What a window holds of the mod list and of Fleet's word rules, over a `window.armada` of just those. */
function wordRulesFrom(rulesFor: (root: string, sessionId: string | undefined) => { address: string }[], mods: object[] = [{ name: "jira-keys", kind: "words", enabled: true, valid: true, changed_at: "t1" }]) {
  const listWordRules = vi.fn(async (root: string, sessionId?: string) => ({
    ok: true as const,
    value: { rules: rulesFor(root, sessionId).map((one) => ({ mod_name: "jira-keys", name: "Jira key", pattern: "[A-Z][A-Z0-9]+-\\d+", example: "STORE-1234", ...one })) },
  }));
  (window as unknown as { armada: unknown }).armada = {
    syncState: async () => ({ full: { mods: { mods } }, seq: 1 }),
    subscribeChanges: () => undefined,
    findPanelFiles: async () => [],
    listWordRules,
  };
  return listWordRules;
}

const REPOSITORIES = [
  { root: "/code/store", records_root: "/records/store", manifest: { id: "store" } },
  { root: "/code/payments", records_root: "/records/payments", manifest: { id: "payments" } },
] as unknown as Host["repositories"];

const KEY = "STORE-1234";
const words = (refs: ProseReferences, text: string) => refs.scan(text).map((one) => [text.slice(one.from, one.to), one.ref.id]);

/** A Session's thread in `manifestId`, drawn until Fleet's rules for its repository have arrived. */
async function threadIn(manifestId: string, id: string, text: string) {
  const thread = { ...session(id, `Job in ${manifestId}`), manifestId } as Session;
  const { seen, render } = draw([thread], thread);
  await act(async () => render({ ...hostOf(() => undefined), repositories: REPOSITORIES }));
  await act(async () => {
    await vi.waitFor(() => expect(words(seen.at(-1)!, text)).not.toEqual([]));
  });
  return seen.at(-1)!;
}

test("STORE-1234 links to the repository's own address in each Session, and asks Fleet for the Session it is drawn in", async () => {
  const list = wordRulesFrom((root) => [{ address: root === "/code/store" ? "https://store.atlassian.net/browse/{match}" : "https://pay.atlassian.net/browse/{match}" }]);
  const store = await threadIn("store", "01STOREJOB0000000000000000", `Cite ${KEY}`);
  const payments = await threadIn("payments", "01PAYMENTSJOB00000000000000", `Cite ${KEY}`);
  expect(words(store, `Cite ${KEY}`)).toEqual([[KEY, "https://store.atlassian.net/browse/STORE-1234"]]);
  expect(words(payments, `Cite ${KEY}`)).toEqual([[KEY, "https://pay.atlassian.net/browse/STORE-1234"]]);
  expect(list).toHaveBeenCalledWith("/code/store", "01STOREJOB0000000000000000");
  expect(list).toHaveBeenCalledWith("/code/payments", "01PAYMENTSJOB00000000000000");
});

test("UTF-8 beside STORE-1234 stays plain, and the link's span counts the text as it is written", async () => {
  wordRulesFrom(() => [{ address: "https://store.atlassian.net/browse/{match}" }]);
  const text = "😀 café 日本語 STORE-1234 naïve";
  const refs = await threadIn("store", "01STOREJOB0000000000000000", text);
  expect(words(refs, text)).toEqual([[KEY, "https://store.atlassian.net/browse/STORE-1234"]]);
  expect(refs.scan(text)).toHaveLength(1);
});

test("a pattern with no address for the repository draws plain there, and where there is one it links", async () => {
  wordRulesFrom((root) => (root === "/code/store" ? [{ address: "https://store.atlassian.net/browse/{match}" }] : []));
  const payments = { ...session("01PAYMENTSJOB00000000000000", "Job in payments"), manifestId: "payments" } as Session;
  const { seen, render } = draw([payments], payments);
  await act(async () => render({ ...hostOf(() => undefined), repositories: REPOSITORIES }));
  await act(async () => new Promise((done) => setTimeout(done, 50)));
  expect(words(seen.at(-1)!, `Cite ${KEY}`)).toEqual([]);
});

test("a Words mod that is switched off asks Fleet for nothing, and links nothing", async () => {
  const list = wordRulesFrom(() => [{ address: "https://store.atlassian.net/browse/{match}" }], [{ name: "jira-keys", kind: "words", enabled: false, valid: true }]);
  const thread = { ...session("01STOREJOB0000000000000000", "Job in store"), manifestId: "store" } as Session;
  const { seen, render } = draw([thread], thread);
  await act(async () => render({ ...hostOf(() => undefined), repositories: REPOSITORIES }));
  await act(async () => new Promise((done) => setTimeout(done, 50)));
  expect(list).not.toHaveBeenCalled();
  expect(words(seen.at(-1)!, `Cite ${KEY}`)).toEqual([]);
});

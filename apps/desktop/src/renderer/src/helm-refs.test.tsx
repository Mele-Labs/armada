// What a thread's links hold across redraws: the same `ProseRefs` while nothing they can link has changed, since a new one redraws
// every message in the thread and reads each of them again. A press still goes to the latest `openSession`.

import { act, useContext } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { ProseRefs } from "@armada/components";
import type { ProseReferences } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../shared/bridge";
import { RefsHost, SessionRefs } from "./helm-refs";
import type { Host } from "./helm-refs";
import { SessionsFrom } from "./sessions-draft";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
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
  (window as unknown as { armada: unknown }).armada = { findPanelFiles: find };
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

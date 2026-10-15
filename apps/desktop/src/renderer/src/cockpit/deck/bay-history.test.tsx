// A Bay pane draws the timeline's ledger until Fleet answers, then Fleet's answer, and asks again when the ledger moves.
// Browser: a hook is read through the window it renders in.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import type { BayHistory } from "@armada/protocol";

import type { BayHistoryRead } from "../../../../shared/api/bay-history";
import { useBayHistory } from "./bay-history";
import type { Ledger } from "./bays";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
});

const HOLDER = { kind: "session", id: "s1", title: "Frame" } as const;
const ledgerOf = (...ends: (number | undefined)[]): Ledger => ({ leases: ends.map((to, index) => ({ holder: HOLDER, from: 1_000 + index, to })), first: 1_000 });
const served: BayHistory = {
  manifest_id: "m1",
  slot: 3,
  entries: [{ kind: "checkout", at: "2026-10-14T10:00:00Z", to: "main", from: "fix/x", sha: "9a1c77e" }],
};

function Probe({ ledger, read }: { ledger: Ledger; read: (manifestId: string, slot: number) => Promise<BayHistoryRead> }) {
  const history = useBayHistory("m1", 3, ledger, read);
  return <p>{history === undefined ? "none" : history.entries.map((one) => one.kind).join(",")}</p>;
}

function mount() {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  return { host, root };
}

test("Fleet's answer replaces the ledger, and a slot event asks again", async () => {
  const asked: number[] = [];
  let answer: BayHistoryRead = { ok: true, value: served };
  const read = async (_manifestId: string, slot: number) => {
    asked.push(slot);
    return answer;
  };
  const { host, root } = mount();

  await act(async () => root.render(<Probe ledger={ledgerOf(2_000)} read={read} />));
  expect(host.textContent).toBe("checkout");
  expect(asked).toEqual([3]);

  // The same ledger asks nothing more; a lease that arrives asks again.
  await act(async () => root.render(<Probe ledger={ledgerOf(2_000)} read={read} />));
  expect(asked).toHaveLength(1);
  answer = { ok: true, value: { ...served, entries: [...served.entries, { kind: "released", at: "2026-10-14T11:00:00Z", holder: HOLDER }] } };
  await act(async () => root.render(<Probe ledger={ledgerOf(2_000, undefined)} read={read} />));
  expect(asked).toHaveLength(2);
  expect(host.textContent).toBe("checkout,released");
});

test("a refused answer leaves the ledger's history", async () => {
  const refused = async (): Promise<BayHistoryRead> => ({ ok: false, outcome: { ok: false, why: "transport" } as never });
  const { host, root } = mount();
  await act(async () => root.render(<Probe ledger={ledgerOf(2_000)} read={refused} />));
  expect(host.textContent).toBe("released,leased");
});

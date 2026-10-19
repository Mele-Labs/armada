// The one side panel Armada opens a pull request, an issue, a bay or a Check in. **A surface asks it with `openInPanel`**
// and it answers whether a panel is drawn at all: where nothing hosts one the answer is `false` and the surface opens
// the thing as it always has. The host (`host.tsx`) is mounted where a window serves the reads the panels draw from.

import { useSyncExternalStore } from "react";

/** What a press hands over: enough to name the thing, and whatever the surface already knew of it. The panel reads the rest. */
export type PanelItem =
  | { kind: "pr"; number: number; address: string; repository?: string; manifestId?: string; root?: string; title?: string; branch?: string; tab?: string }
  | { kind: "issue"; number: number; root?: string; manifestId?: string; address?: string; title?: string }
  | { kind: "bay"; manifestId: string; slot: number }
  | { kind: "check"; id: string; name?: string; branch?: string; land?: { root: string; branch: string; check: string }; row?: { state: "passed" | "failed" | "running"; seconds?: number; address?: string } };

type Held = { open: PanelItem | undefined; back: PanelItem | undefined; hosts: number };

let held: Held = { open: undefined, back: undefined, hosts: 0 };
const listeners = new Set<() => void>();
const set = (next: Partial<Held>) => {
  held = { ...held, ...next };
  listeners.forEach((one) => one());
};

/** A window that draws the panel says so while it is mounted. */
export function hostsPanel(): () => void {
  set({ hosts: held.hosts + 1 });
  return () => set({ hosts: held.hosts - 1, ...(held.hosts === 1 ? { open: undefined, back: undefined } : {}) });
}

let browse: ((address: string) => boolean) | undefined;

/** Where `links.openPullRequestsIn` says the browser, the host hands over how a pull request goes there. */
export function browsePullRequestsWith(next: ((address: string) => boolean) | undefined): () => void {
  browse = next;
  return () => {
    if (browse === next) browse = undefined;
  };
}

/** Opens the panel on `item`, and answers whether one is hosted. Opened from the panel itself, the one it was on is the way back. */
export function openInPanel(item: PanelItem): boolean {
  if (held.hosts === 0) return false;
  if (item.kind === "pr" && item.address !== "" && browse?.(item.address) === true) return true;
  set({ open: item, back: held.open !== undefined && held.open.kind !== item.kind ? held.open : held.back });
  return true;
}

/** Back to the item the panel was on before a press inside it moved it. */
export function panelBack(): void {
  if (held.back !== undefined) set({ open: held.back, back: undefined });
}

export function closePanel(): void {
  set({ open: undefined, back: undefined });
}

/** A pull request's address as the forge writes it: `/<owner>/<name>/pull/<number>`, with nothing after the number. */
const PULL = /^https?:\/\/[^/]+\/(?:[^/]+\/)+pull\/(\d+)\/?$/;

/** Opens the pull request an address names, where one is hosted. Any other address is left alone. */
export function openAddressInPanel(address: string, tab?: string): boolean {
  const number = PULL.exec(address)?.[1];
  return number === undefined ? false : openInPanel({ kind: "pr", number: Number(number), address, ...(tab === undefined ? {} : { tab }) });
}

/** A bay's key on the deck, `slot:<manifest>/<slot>`. */
export function openSlotKeyInPanel(key: string): boolean {
  const rest = key.startsWith("slot:") ? key.slice("slot:".length) : undefined;
  const at = rest?.lastIndexOf("/") ?? -1;
  if (rest === undefined || at < 0 || Number.isNaN(Number(rest.slice(at + 1)))) return false;
  return openInPanel({ kind: "bay", manifestId: rest.slice(0, at), slot: Number(rest.slice(at + 1)) });
}

export const usePanel = (): Pick<Held, "open" | "back"> => {
  const now = useSyncExternalStore(
    (listener) => (listeners.add(listener), () => listeners.delete(listener)),
    () => held,
  );
  return now;
};

/** A window starting over: nothing open, and nothing hosted until its own host mounts. */
export function forgetPanel(): void {
  held = { open: undefined, back: undefined, hosts: 0 };
}

/** The deck's pull request, as the panel names it. */
export const openPullRequestInPanel = (pr: { number: number; url: string; root: string; title: string; branch: string }): boolean =>
  openInPanel({ kind: "pr", number: pr.number, address: pr.url, root: pr.root, title: pr.title, branch: pr.branch });

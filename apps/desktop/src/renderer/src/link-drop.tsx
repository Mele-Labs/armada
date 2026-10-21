// A link dropped anywhere in Bridge: pasted outside a text box, or dragged onto the window, it opens the link panel over
// the page; pasted into a text box, it goes in as typed and an offer to open the panel stands beside the box for a few
// seconds. **The one place paste and drop are heard for a link**, at the shell, so no surface listens for links itself.
// A package pasted or dropped is `sharing/ImportPackage.tsx`'s, which listens beside this one; the two never take the same text.
// Kit's own Add opens the same panel, empty. Mock only until Fleet answers `lookAtKitSource`: without it nothing here listens.

import { useEffect, useSyncExternalStore } from "react";

import { LINK_OFFER_ANCHOR, LinkOffer, LinkPanel } from "@armada/components";
import type { KitInstallAnswer, KitLookAnswer, LinkJobAnswer } from "@armada/components";
import type { KitSourceFound } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import { kitChanged, kitSourceJobMessage, kitSourceJobRequest } from "@armada/screens/src/manifest-kit";

import { useDeck } from "./cockpit/deck/store";
import { useSessionsDraft } from "./sessions-draft";

type Held = { open: boolean; source: string; opened: number; offer: { field: HTMLElement; link: string } | null };

let held: Held = { open: false, source: "", opened: 0, offer: null };
const listeners = new Set<() => void>();
const set = (next: Partial<Held>) => {
  held = { ...held, ...next };
  listeners.forEach((one) => one());
};
const subscribe = (on: () => void) => {
  listeners.add(on);
  return () => {
    listeners.delete(on);
  };
};

const NEVER = () => () => undefined;

/** How long the offer beside a text box stands. */
const OFFER_MS = 6000;
let offerTimer: number | undefined;

function dropOffer(): void {
  window.clearTimeout(offerTimer);
  held.offer?.field.style.removeProperty("anchor-name");
  if (held.offer !== null) set({ offer: null });
}

/** Opens the panel over whatever page is showing, looking at `source` at once where there is one. */
export function openLinkPanel(source: string): void {
  dropOffer();
  set({ open: true, source, opened: held.opened + 1 });
}

/** Whether the link panel can open in this window: only where something answers what a link is. */
export const linkPanelServed = (): boolean => window.armada.lookAtKitSource !== undefined;

/** The text, where it is one link and nothing else. Plain words, and `owner/repository` on its own, are not. */
export function linkOf(text: string | undefined): string | null {
  const one = text?.trim() ?? "";
  if (!/^https?:\/\/\S+$/i.test(one)) return null;
  try {
    return new URL(one).protocol.startsWith("http") ? one : null;
  } catch {
    return null;
  }
}

/** A field a person types into, where a paste is theirs. */
function fieldOf(target: EventTarget | null): HTMLElement | null {
  if (!(target instanceof HTMLElement)) return null;
  const field = target.closest<HTMLElement>("input, textarea, [contenteditable=''], [contenteditable='true']");
  if (field instanceof HTMLInputElement && !["text", "search", "url", ""].includes(field.type)) return null;
  return field;
}

/** Listens on the window for a link pasted or dropped. Mounted once, by `App`. */
export function useLinkDrop(): void {
  useEffect(() => {
    if (!linkPanelServed()) return;
    // A paste or drop something else took, a package (`sharing/ImportPackage.tsx`), is left to it.
    const onPaste = (event: ClipboardEvent) => {
      if (event.defaultPrevented) return;
      const link = linkOf(event.clipboardData?.getData("text/plain"));
      if (link === null) return;
      const field = fieldOf(event.target);
      if (field === null) {
        event.preventDefault();
        openLinkPanel(link);
        return;
      }
      // The panel's own field takes a link as a link: no offer to open what is open.
      if (field.closest(".armada-link-panel") !== null) return;
      dropOffer();
      field.style.setProperty("anchor-name", LINK_OFFER_ANCHOR);
      set({ offer: { field, link } });
      offerTimer = window.setTimeout(dropOffer, OFFER_MS);
    };
    const linkDragged = (event: DragEvent) => event.dataTransfer?.types.includes("text/uri-list") === true;
    // A link dragged over the window is taken by it, rather than navigating it away, except over a field it can go into.
    const onOver = (event: DragEvent) => {
      if (linkDragged(event) && fieldOf(event.target) === null) event.preventDefault();
    };
    const onDrop = (event: DragEvent) => {
      if (event.defaultPrevented || fieldOf(event.target) !== null) return;
      const link = linkOf(event.dataTransfer?.getData("text/uri-list").split("\n").find((one) => !one.startsWith("#")) ?? event.dataTransfer?.getData("text/plain"));
      if (link === null) return;
      event.preventDefault();
      openLinkPanel(link);
    };
    window.addEventListener("paste", onPaste);
    window.addEventListener("dragover", onOver);
    window.addEventListener("drop", onDrop);
    return () => {
      window.removeEventListener("paste", onPaste);
      window.removeEventListener("dragover", onOver);
      window.removeEventListener("drop", onDrop);
      dropOffer();
    };
  }, []);
}

/** The panel and the offer, drawn once over every page. */
export function LinkPanelHost() {
  const now = useSyncExternalStore(subscribe, () => held);
  const sessions = useSessionsDraft();
  const sel = useDeck((deck) => deck.sel);
  const list = useSyncExternalStore(sessions?.subscribe ?? NEVER, () => sessions?.get());
  const look = window.armada.lookAtKitSource;
  const install = window.armada.installKitPiece;
  if (look === undefined || install === undefined) return null;

  const openId = sel?.kind === "session" ? sel.key.slice("session:".length) : undefined;
  const open = openId === undefined ? undefined : list?.find((one) => one.id === openId);

  const onLook = async (source: string): Promise<KitLookAnswer> => {
    const read = await look({ source });
    return read.ok ? { state: "found", found: read.found } : { state: "refused", why: said(read.outcome) };
  };
  const onInstall = async (adding: { source: string; kind: "plugin" | "skill"; manifest: boolean }): Promise<KitInstallAnswer> => {
    const read = await install(adding);
    if (!read.ok) return { state: "refused", why: said(read.outcome) };
    if (read.installed.state === "failed") {
      const { command, exit_code, error } = read.installed;
      return { state: "failed", command, exitCode: exit_code, error };
    }
    kitChanged();
    return { state: "installed" };
  };
  const onStartJob = async (source: string): Promise<LinkJobAnswer> => {
    if (sessions === undefined) return { state: "refused", why: "No Jobs are served in this window." };
    const id = await sessions.start(undefined, undefined, kitSourceJobRequest(source));
    const started = id === undefined ? undefined : sessions.get().find((one) => one.id === id);
    return started === undefined ? { state: "refused", why: sessions.said?.() ?? "The Job did not start." } : { state: "done", title: started.title };
  };
  const job =
    open === undefined || sessions === undefined
      ? undefined
      : {
          title: open.title,
          onAdd: async (source: string, found: KitSourceFound | undefined): Promise<LinkJobAnswer> => {
            const went = await sessions.send(open.id, { text: kitSourceJobMessage(source, found), files: [], sketches: [], tags: [] });
            return went === false ? { state: "refused", why: sessions.said?.() ?? "The message did not go." } : { state: "done", title: open.title };
          },
        };

  return (
    <>
      <LinkPanel key={now.opened} open={now.open} source={now.source} onClose={() => set({ open: false })} onLook={onLook} onInstall={onInstall} onStartJob={onStartJob} job={job} />
      {now.offer === null ? null : <LinkOffer onOpen={() => openLinkPanel(now.offer!.link)} />}
    </>
  );
}

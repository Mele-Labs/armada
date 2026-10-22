// A link dropped anywhere in Bridge: pasted outside a text box, or dragged onto the window, it opens the link panel over
// the page; pasted into a text box, it goes in as typed and an offer to open the panel stands beside the box for a few
// seconds. **The one place a link is claimed**, from the shell's one listener (`paste-and-drop.ts`), so no surface listens
// for links itself. A package pasted or dropped is offered to `sharing/ImportPackage.tsx` first.
// Kit's own Add opens the same panel, empty.

import { useEffect, useSyncExternalStore } from "react";

import { LINK_OFFER_ANCHOR, LinkOffer, LinkPanel } from "@armada/components";
import type { KitInstallAnswer, KitLookAnswer, LinkJobAnswer } from "@armada/components";
import type { KitSourceFound } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import { kitChanged, kitInstallFixRequest, kitSourceJobMessage, kitSourceJobRequest } from "@armada/screens/src/manifest-kit";

import { installKitPiece, lookAtKitSource } from "./commands";
import { useDeck } from "./cockpit/deck/store";
import { askToOpenSession } from "./open-session";
import { useClaim } from "./paste-and-drop";
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

/** Claims a link pasted or dropped, after a package has had its turn (`paste-and-drop.ts`). Mounted once, by `App`. */
export function useLinkDrop(): void {
  const linkDragged = (event: DragEvent) => event.dataTransfer?.types.includes("text/uri-list") === true;
  useClaim("link", {
    paste: (event) => {
      const link = linkOf(event.clipboardData?.getData("text/plain"));
      if (link === null) return false;
      const field = fieldOf(event.target);
      if (field === null) {
        event.preventDefault();
        openLinkPanel(link);
        return true;
      }
      // The panel's own field takes a link as a link: no offer to open what is open.
      if (field.closest(".armada-link-panel") !== null) return false;
      dropOffer();
      field.style.setProperty("anchor-name", LINK_OFFER_ANCHOR);
      set({ offer: { field, link } });
      offerTimer = window.setTimeout(dropOffer, OFFER_MS);
      return true;
    },
    // A link dragged over the window is taken by it, rather than navigating it away, except over a field it can go into.
    over: (event) => {
      if (!linkDragged(event) || fieldOf(event.target) !== null) return false;
      event.preventDefault();
      return true;
    },
    drop: (event) => {
      if (fieldOf(event.target) !== null) return false;
      const link = linkOf(event.dataTransfer?.getData("text/uri-list").split("\n").find((one) => !one.startsWith("#")) ?? event.dataTransfer?.getData("text/plain"));
      if (link === null) return false;
      event.preventDefault();
      openLinkPanel(link);
      return true;
    },
  });
  useEffect(() => dropOffer, []);
}

/** The panel and the offer, drawn once over every page. */
export function LinkPanelHost() {
  const now = useSyncExternalStore(subscribe, () => held);
  const sessions = useSessionsDraft();
  const sel = useDeck((deck) => deck.sel);
  const list = useSyncExternalStore(sessions?.subscribe ?? NEVER, () => sessions?.get());

  const openId = sel?.kind === "session" ? sel.key.slice("session:".length) : undefined;
  const open = openId === undefined ? undefined : list?.find((one) => one.id === openId);

  const onLook = async (source: string): Promise<KitLookAnswer> => {
    const read = await lookAtKitSource({ source });
    return read.ok ? { state: "found", found: read.found } : { state: "refused", why: said(read.outcome) };
  };
  const onInstall = async (adding: { source: string; kind: "plugin" | "skill"; choice?: string }): Promise<KitInstallAnswer> => {
    const read = await installKitPiece(adding);
    if (!read.ok) return { state: "refused", why: said(read.outcome) };
    if (read.installed.state === "failed") {
      const { command, exit_code, error, taken_back } = read.installed;
      return { state: "failed", command, exitCode: exit_code, error, takenBack: taken_back };
    }
    kitChanged();
    return { state: "installed" };
  };
  const onStartJob = async (source: string, failed?: { command: string; error: string }): Promise<LinkJobAnswer> => {
    if (sessions === undefined) return { state: "refused", why: "No Jobs are served in this window." };
    const id = await sessions.start(undefined, undefined, failed === undefined ? kitSourceJobRequest(source) : kitInstallFixRequest(source, failed));
    const started = id === undefined ? undefined : sessions.get().find((one) => one.id === id);
    return id === undefined ? { state: "refused", why: sessions.said?.() ?? "The Job did not start." } : { state: "done", id, title: started?.title };
  };
  const job =
    open === undefined || sessions === undefined
      ? undefined
      : {
          title: open.title,
          onAdd: async (source: string, found: KitSourceFound | undefined): Promise<LinkJobAnswer> => {
            const went = await sessions.send(open.id, { text: kitSourceJobMessage(source, found), files: [], sketches: [], tags: [] });
            return went === false ? { state: "refused", why: sessions.said?.() ?? "The message did not go." } : { state: "done", id: open.id, title: open.title };
          },
        };

  return (
    <>
      <LinkPanel
        key={now.opened}
        open={now.open}
        source={now.source}
        onClose={() => set({ open: false })}
        onLook={onLook}
        onInstall={onInstall}
        onStartJob={onStartJob}
        job={job}
        onOpenJob={(id) => {
          set({ open: false });
          askToOpenSession(id);
        }}
        titleOf={(id) => list?.find((one) => one.id === id)?.title}
      />
      {now.offer === null ? null : <LinkOffer onOpen={() => openLinkPanel(now.offer!.link)} />}
    </>
  );
}

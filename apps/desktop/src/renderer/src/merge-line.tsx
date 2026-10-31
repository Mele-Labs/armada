// The merge line, as the window draws it: panels under Overview's lists, and a rail surface of its
// own. **The same panels in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **One panel per repository Fleet serves a line for**, off `mergeLineViews`: the picked one, or
// every one on All, named once there is more than one. With none served for the pick, neither the
// panels nor the rail row draws, rather than a sentence about an absence or a row opening nothing.

import { useEffect, useMemo, useRef } from "react";

import type { RepositorySummary } from "@armada/protocol";
import { MergeLine, useTileGrid, type FixChoice } from "@armada/components";
import { mergeLineViews, recentSessionsOf, type MergeLineView } from "@armada/screens";
import { Boundary, SURFACE } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { PrOwnersOnTheWire } from "./pr-owners-wire";
import { useDispatching } from "./dispatch-panel/dispatching";
import { usePanelOpen } from "./panel-open";
import { useSessions } from "./sessions-draft";
import { openAddressInPanel, openInPanel } from "./shared-panel/store";

type Lined = Pick<BridgeState, "mergeLines" | "repository" | "holds" | "landFollowed">;

export function viewsOf({ mergeLines, repository, holds }: Lined): readonly MergeLineView[] {
  const repositories: readonly RepositorySummary[] = holds.repositories ?? [];
  return mergeLineViews(mergeLines, repository, repositories);
}

/** The surfaces the rail and the palette leave off: the merge line's, until Fleet serves one. */
export function hiddenSurfaces(state: Lined): readonly string[] {
  return viewsOf(state).length === 0 ? [SURFACE.mergeLine] : [];
}

/** A panel for each line there is. A Check on any of them opens in the side panel. */
export function MergeLinePanel({
  state,
  onOpenLink,
  onOpenSession,
  onSendToSession,
  focus,
  surface = false,
}: {
  state: Lined;
  onOpenLink: (address: string) => void;
  /** A Session the head names. Absent, it is not a press. */
  onOpenSession?: (sessionId: string) => void;
  /** Sends main's red to a Session as a message. */
  onSendToSession?: (sessionId: string, message: string) => void;
  focus?: string;
  /** On the rail's own page, where the band and horizon stay put while the tiles scroll. */
  surface?: boolean;
}) {
  const views = viewsOf(state);
  // One panel fills the page and scrolls its glass; several share the page's scroll, each band
  // holding at the top while its own tiles pass, so no panel is squeezed into a slot of its own.
  const pinned = !surface ? undefined : views.length === 1 ? "glass" : "page";
  return (
    <PrOwnersOnTheWire lines={state.mergeLines}>
      {views.map((view) => (
        <OneLine
          key={view.root}
          view={view}
          {...(pinned === undefined ? {} : { pinned })}
          {...(focus === undefined ? {} : { focus })}
          {...(onOpenSession === undefined ? {} : { onOpenSession })}
          {...(onSendToSession === undefined ? {} : { onSendToSession })}
          onOpenLink={onOpenLink}
          onOpenCheck={(branch, check) => void openInPanel({ kind: "check", id: `land:${view.root}:${branch}:${check}`, name: check, branch, land: { root: view.root, branch, check } })}
        />
      ))}
    </PrOwnersOnTheWire>
  );
}

/**
 * What the band's choices do for one repository: a Session is sent the work as a message; a new Session is drafted and opened on
 * the brief. **The Merge line's own routes, held once so the Cockpit's card sends the same ones.** Absent where the window holds no
 * Dispatch panel, so the band offers none.
 */
export function useFixChoice(
  root: string,
  onSendToSession: ((sessionId: string, message: string) => void) | undefined,
): ((choice: FixChoice) => void) | undefined {
  const dispatching = useDispatching();
  if (dispatching === undefined) return undefined;
  return (choice) => {
    if (choice.kind === "session") onSendToSession?.(choice.session, choice.message);
    else dispatching.startDraft(choice.request, root);
  };
}

/** One repository's panel, folded on its own. */
function OneLine({
  view,
  focus,
  onOpenLink,
  onOpenSession,
  onSendToSession,
  onOpenCheck,
  pinned,
}: {
  view: MergeLineView;
  pinned?: "glass" | "page";
  /** A branch another surface asked for. The panel opens if it holds it. */
  focus?: string;
  onOpenLink: (address: string) => void;
  onOpenSession?: (sessionId: string) => void;
  onSendToSession?: (sessionId: string, message: string) => void;
  onOpenCheck: (branch: string, check: string) => void;
}) {
  const [open, setOpen] = usePanelOpen(`merge-line:${view.root}`);
  // The Sessions work can be sent back to are the ones the window already holds, the one that merged among them.
  const sessions = useSessions();
  const fix = useFixChoice(view.root, onSendToSession);
  const culprit = view.hub?.main?.state === "red" ? view.hub.main.red.merge?.session : undefined;
  const hub = useMemo(() => (view.hub === undefined ? undefined : { ...view.hub, recentSessions: recentSessionsOf(sessions, culprit) }), [view.hub, sessions, culprit]);
  const holds = focus !== undefined && [...view.line, ...view.landed, ...view.sentBack, ...(view.hub?.pulls ?? [])].some((one) => one.branch === focus);
  useEffect(() => {
    // Only when the ask arrives: `setOpen` is not stable, and a fold the person closes stays closed.
    if (holds) setOpen(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [holds, focus]);
  return (
    <MergeLine
      name={view.name}
      line={view.line}
      landed={view.landed}
      sentBack={view.sentBack}
      notice={view.notice}
      {...(hub === undefined ? {} : { hub })}
      {...(onOpenSession === undefined ? {} : { onOpenSession })}
      open={open}
      {...(pinned === undefined ? {} : { pinned })}
      {...(holds ? { focus } : {})}
      {...(fix === undefined ? {} : { onFix: fix })}
      onOpenChange={setOpen}
      onOpenPullRequest={(address) => {
        if (!openAddressInPanel(address)) onOpenLink(address);
      }}
      onOpenCheck={onOpenCheck}
    />
  );
}

/** The rail surface: the panels alone, on Overview's own padding. */
export function MergeLineSurface({
  state,
  bridge,
  onCopied,
  onOpenLink,
  onOpenSession,
  onSendToSession,
  focus,
}: {
  state: Lined;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
  onOpenSession?: (sessionId: string) => void;
  onSendToSession?: (sessionId: string, message: string) => void;
  focus?: string;
}) {
  // One Tab stop for every repository's tiles, the arrows across them all. Enter on a tile leaves the press alone.
  const scope = useRef<HTMLDivElement>(null);
  useTileGrid(scope, { tile: "[data-merge-item]", onOpen: () => false });
  return (
    <Boundary region="Merge line" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview" ref={scope}>
        <MergeLinePanel
          surface
          state={state}
          onOpenLink={onOpenLink}
          {...(onOpenSession === undefined ? {} : { onOpenSession })}
          {...(onSendToSession === undefined ? {} : { onSendToSession })}
          {...(focus === undefined ? {} : { focus })}
        />
      </div>
    </Boundary>
  );
}

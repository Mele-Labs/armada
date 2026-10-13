// The merge line, as the window draws it: panels under Overview's lists, and a rail surface of its
// own. **The same panels in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **One panel per repository Fleet serves a line for**, off `mergeLineViews`: the picked one, or
// every one on All, named once there is more than one. With none served for the pick, neither the
// panels nor the rail row draws, rather than a sentence about an absence or a row opening nothing.

import { useEffect, useMemo, useRef, useState } from "react";

import type { FixMain, LandCheckAt, RepositorySummary } from "@armada/protocol";
import { MergeLine, useTileGrid } from "@armada/components";
import { mergeLineViews, recentSessionsOf, type MergeLineView } from "@armada/screens";
import { LandCheckLogSheet } from "@armada/jobs";
import { Boundary, SURFACE, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { followLandCheck } from "./commands";
import { usePanelOpen } from "./panel-open";
import { useSessions } from "./sessions-draft";

type Lined = Pick<BridgeState, "mergeLines" | "repository" | "holds" | "landFollowed" | "jobs">;

export function viewsOf({ mergeLines, repository, holds, jobs }: Lined): readonly MergeLineView[] {
  const repositories: readonly RepositorySummary[] = holds.repositories ?? [];
  return mergeLineViews(mergeLines, repository, repositories, jobs);
}

/** The surfaces the rail and the palette leave off: the merge line's, until Fleet serves one. */
export function hiddenSurfaces(state: Lined): readonly string[] {
  return viewsOf(state).length === 0 ? [SURFACE.mergeLine] : [];
}

/**
 * A panel for each line there is, and the log panel a Check on any of them opens in: **one, held
 * here**, so a second press replaces the first rather than stacking a panel per line.
 */
export function MergeLinePanel({
  state,
  onOpenLink,
  onOpenJob,
  onOpenSession,
  onFix,
  onSendToSession,
  focus,
  surface = false,
}: {
  state: Lined;
  onOpenLink: (address: string) => void;
  /** A Job the head or a pull request names. Absent, it is not a press. */
  onOpenJob?: (jobId: string) => void;
  /** A Session the head names. Absent, it is not a press. */
  onOpenSession?: (sessionId: string) => void;
  /** Hands main's red to a Job. Absent, the band offers no way to. */
  onFix?: (fix: FixMain) => void;
  /** Sends main's red to a Session as a message. */
  onSendToSession?: (sessionId: string, message: string) => void;
  focus?: string;
  /** On the rail's own page, where the band and horizon stay put while the tiles scroll. */
  surface?: boolean;
}) {
  const [reading, setReading] = useState<LandCheckAt | null>(null);
  const floor = useAtFloor();
  const views = viewsOf(state);
  // One panel fills the page and scrolls its glass; several share the page's scroll, each band
  // holding at the top while its own tiles pass, so no panel is squeezed into a slot of its own.
  const pinned = !surface ? undefined : views.length === 1 ? "glass" : "page";
  return (
    <>
      {views.map((view) => (
        <OneLine
          key={view.root}
          view={view}
          {...(pinned === undefined ? {} : { pinned })}
          {...(focus === undefined ? {} : { focus })}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onOpenSession === undefined ? {} : { onOpenSession })}
          {...(onFix === undefined ? {} : { onFix })}
          {...(onSendToSession === undefined ? {} : { onSendToSession })}
          onOpenLink={onOpenLink}
          onOpenCheck={(branch, check) => setReading({ root: view.root, branch, check })}
        />
      ))}
      {reading === null ? null : (
        <LandCheckLogSheet
          at={reading}
          followed={state.landFollowed}
          onFollow={followLandCheck}
          floor={floor}
          onClose={() => setReading(null)}
        />
      )}
    </>
  );
}

/** One repository's panel, folded on its own. */
function OneLine({
  view,
  focus,
  onOpenLink,
  onOpenJob,
  onOpenSession,
  onFix,
  onSendToSession,
  onOpenCheck,
  pinned,
}: {
  view: MergeLineView;
  pinned?: "glass" | "page";
  /** A branch another surface asked for. The panel opens if it holds it. */
  focus?: string;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  onOpenSession?: (sessionId: string) => void;
  onFix?: (fix: FixMain) => void;
  onSendToSession?: (sessionId: string, message: string) => void;
  onOpenCheck: (branch: string, check: string) => void;
}) {
  const [open, setOpen] = usePanelOpen(`merge-line:${view.root}`);
  // The Sessions work can be sent back to are the ones the window already holds, the one that merged among them.
  const sessions = useSessions();
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
      {...(onOpenJob === undefined ? {} : { onOpenJob })}
      {...(onOpenSession === undefined ? {} : { onOpenSession })}
      open={open}
      {...(pinned === undefined ? {} : { pinned })}
      {...(holds ? { focus } : {})}
      {...(onFix === undefined
        ? {}
        : {
            onFix: (choice) =>
              choice.kind === "session"
                ? onSendToSession?.(choice.session, choice.message)
                : onFix({ root: view.root, ...(choice.kind === "back" ? { job: choice.job } : { brief: choice.request }) }),
          })}
      onOpenChange={setOpen}
      onOpenPullRequest={onOpenLink}
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
  onOpenJob,
  onOpenSession,
  onFix,
  onSendToSession,
  focus,
}: {
  state: Lined;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  onOpenSession?: (sessionId: string) => void;
  onFix?: (fix: FixMain) => void;
  onSendToSession?: (sessionId: string, message: string) => void;
  focus?: string;
}) {
  // One Tab stop for every repository's tiles, the arrows across them all. Enter on a tile, and `o`
  // anywhere in one, open the Job it came from; a tile with no Job leaves the press alone.
  const scope = useRef<HTMLDivElement>(null);
  useTileGrid(scope, {
    tile: "[data-merge-item]",
    onOpen: (tile) => {
      const job = tile.dataset.job;
      if (job === undefined || onOpenJob === undefined) return false;
      onOpenJob(job);
    },
  });
  return (
    <Boundary region="Merge line" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview" ref={scope}>
        <MergeLinePanel
          surface
          state={state}
          onOpenLink={onOpenLink}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onOpenSession === undefined ? {} : { onOpenSession })}
          {...(onFix === undefined ? {} : { onFix })}
          {...(onSendToSession === undefined ? {} : { onSendToSession })}
          {...(focus === undefined ? {} : { focus })}
        />
      </div>
    </Boundary>
  );
}

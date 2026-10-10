// The merge line, as the window draws it: panels under Overview's lists, and a rail surface of its
// own. **The same panels in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **One panel per repository Fleet serves a line for**, off `mergeLineViews`: the picked one, or
// every one on All, named once there is more than one. With none served for the pick, neither the
// panels nor the rail row draws, rather than a sentence about an absence or a row opening nothing.

import { useEffect, useRef, useState } from "react";

import type { FixMain, LandCheckAt, RepositorySummary } from "@armada/protocol";
import { MergeLine, useTileGrid } from "@armada/components";
import { mergeLineViews, type MergeLineView } from "@armada/screens";
import { LandCheckLogSheet } from "@armada/jobs";
import { Boundary, SURFACE, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { followLandCheck } from "./commands";
import { usePanelOpen } from "./panel-open";

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
  onFix,
  focus,
}: {
  state: Lined;
  onOpenLink: (address: string) => void;
  /** A Job the head or a pull request names. Absent, it is not a press. */
  onOpenJob?: (jobId: string) => void;
  /** Hands main's red to a Job. Absent, the band offers no way to. */
  onFix?: (fix: FixMain) => void;
  focus?: string;
}) {
  const [reading, setReading] = useState<LandCheckAt | null>(null);
  const floor = useAtFloor();
  return (
    <>
      {viewsOf(state).map((view) => (
        <OneLine
          key={view.root}
          view={view}
          {...(focus === undefined ? {} : { focus })}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onFix === undefined ? {} : { onFix })}
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
  onFix,
  onOpenCheck,
}: {
  view: MergeLineView;
  /** A branch another surface asked for. The panel opens if it holds it. */
  focus?: string;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  onFix?: (fix: FixMain) => void;
  onOpenCheck: (branch: string, check: string) => void;
}) {
  const [open, setOpen] = usePanelOpen(`merge-line:${view.root}`);
  const holds = focus !== undefined && [...view.line, ...view.landed, ...view.sentBack].some((one) => one.branch === focus);
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
      {...(view.hub === undefined ? {} : { hub: view.hub })}
      {...(onOpenJob === undefined ? {} : { onOpenJob })}
      open={open}
      {...(holds ? { focus } : {})}
      {...(onFix === undefined
        ? {}
        : {
            onFix: (choice) =>
              onFix({
                root: view.root,
                ...(choice.kind === "back" ? { job: choice.job } : { brief: choice.request }),
              }),
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
  onFix,
  focus,
}: {
  state: Lined;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  onFix?: (fix: FixMain) => void;
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
          state={state}
          onOpenLink={onOpenLink}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(onFix === undefined ? {} : { onFix })}
          {...(focus === undefined ? {} : { focus })}
        />
      </div>
    </Boundary>
  );
}

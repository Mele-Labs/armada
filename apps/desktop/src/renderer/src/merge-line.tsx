// The merge line, as the window draws it: panels under Overview's lists, and a rail surface of its
// own. **The same panels in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **One panel per repository Fleet serves a line for**, off `mergeLineViews`: the picked one, or
// every one on All, named once there is more than one. With none served for the pick, neither the
// panels nor the rail row draws, rather than a sentence about an absence or a row opening nothing.

import { useEffect, useState } from "react";

import type { LandCheckAt, RepositorySummary } from "@armada/protocol";
import { MergeLine } from "@armada/components";
import { LandCheckLogSheet, mergeLineViews, type MergeLineView } from "@armada/screens";
import { Boundary, SURFACE, useAtFloor } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { followLandCheck } from "./commands";
import { usePanelOpen } from "./panel-open";

type Lined = Pick<BridgeState, "mergeLines" | "repository" | "holds" | "landFollowed">;

function viewsOf({ mergeLines, repository, holds }: Lined): readonly MergeLineView[] {
  const repositories: readonly RepositorySummary[] = holds.repositories ?? [];
  return mergeLineViews(mergeLines, repository, repositories);
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
  focus,
}: {
  state: Lined;
  onOpenLink: (address: string) => void;
  /** A Job the head or a pull request names. Absent, it is not a press. */
  onOpenJob?: (jobId: string) => void;
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
  onOpenCheck,
}: {
  view: MergeLineView;
  /** A branch another surface asked for. The panel opens if it holds it. */
  focus?: string;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
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
  focus,
}: {
  state: Lined;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
  onOpenJob?: (jobId: string) => void;
  focus?: string;
}) {
  return (
    <Boundary region="Merge line" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        <MergeLinePanel
          state={state}
          onOpenLink={onOpenLink}
          {...(onOpenJob === undefined ? {} : { onOpenJob })}
          {...(focus === undefined ? {} : { focus })}
        />
      </div>
    </Boundary>
  );
}

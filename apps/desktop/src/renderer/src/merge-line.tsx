// The merge line, as the window draws it: panels under Overview's lists, and a rail surface of its
// own. **The same panels in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **One panel per repository Fleet serves a line for**, off `mergeLineViews`: the picked one, or
// every one on All, named once there is more than one. With none served for the pick, neither the
// panels nor the rail row draws, rather than a sentence about an absence or a row opening nothing.

import type { RepositorySummary } from "@armada/protocol";
import { MergeLine } from "@armada/components";
import { mergeLineViews, type MergeLineView } from "@armada/screens";
import { Boundary, SURFACE } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { usePanelOpen } from "./panel-open";

type Lined = Pick<BridgeState, "mergeLines" | "repository" | "holds">;

function viewsOf({ mergeLines, repository, holds }: Lined): readonly MergeLineView[] {
  const repositories: readonly RepositorySummary[] = holds.repositories ?? [];
  return mergeLineViews(mergeLines, repository, repositories);
}

/** The surfaces the rail and the palette leave off: the merge line's, until Fleet serves one. */
export function hiddenSurfaces(state: Lined): readonly string[] {
  return viewsOf(state).length === 0 ? [SURFACE.mergeLine] : [];
}

/** A panel for each line there is. */
export function MergeLinePanel({ state, onOpenLink }: { state: Lined; onOpenLink: (address: string) => void }) {
  return (
    <>
      {viewsOf(state).map((view) => (
        <OneLine key={view.root} view={view} onOpenLink={onOpenLink} />
      ))}
    </>
  );
}

/** One repository's panel, folded on its own. */
function OneLine({ view, onOpenLink }: { view: MergeLineView; onOpenLink: (address: string) => void }) {
  const [open, setOpen] = usePanelOpen(`merge-line:${view.root}`);
  return (
    <MergeLine
      name={view.name}
      line={view.line}
      landed={view.landed}
      sentBack={view.sentBack}
      open={open}
      onOpenChange={setOpen}
      onOpenPullRequest={onOpenLink}
    />
  );
}

/** The rail surface: the panels alone, on Overview's own padding. */
export function MergeLineSurface({
  state,
  bridge,
  onCopied,
  onOpenLink,
}: {
  state: Lined;
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
}) {
  return (
    <Boundary region="Merge line" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        <MergeLinePanel state={state} onOpenLink={onOpenLink} />
      </div>
    </Boundary>
  );
}

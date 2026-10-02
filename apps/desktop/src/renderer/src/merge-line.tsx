// The merge line, as the window draws it: a panel under Overview's lists, and a rail surface of
// its own. **One panel in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **Drawn only where there is a line**, off what Fleet serves (`mergeLineView`): with nobody in
// line, or none for this pick, neither the panel nor the rail row draws, rather than a sentence
// about an absence or a row opening nothing.

import { MergeLine } from "@armada/components";
import { mergeLineView } from "@armada/screens";
import { Boundary, SURFACE } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { usePanelOpen } from "./panel-open";

type Lined = Pick<BridgeState, "mergeLines" | "repository">;

/** The surfaces the rail and the palette leave off: the merge line's, until there is a line. */
export function hiddenSurfaces({ mergeLines, repository }: Lined): readonly string[] {
  return mergeLineView(mergeLines, repository) === undefined ? [SURFACE.mergeLine] : [];
}

/** The panel, where there is a line. */
export function MergeLinePanel({ state, onOpenLink }: { state: Lined; onOpenLink: (address: string) => void }) {
  const mergeLine = mergeLineView(state.mergeLines, state.repository);
  const [open, setOpen] = usePanelOpen("merge-line");
  if (mergeLine === undefined) return null;
  return (
    <MergeLine
      line={mergeLine.line}
      off={mergeLine.off}
      open={open}
      onOpenChange={setOpen}
      onOpenPullRequest={onOpenLink}
    />
  );
}

/** The rail surface: the panel alone, on Overview's own padding. */
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

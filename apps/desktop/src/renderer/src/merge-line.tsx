// The merge line, as the window draws it: a panel under Overview's lists, and a rail surface of
// its own. **One panel in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **Only the mock hands a line over.** Fleet does not serve it yet, so a real Bridge draws no
// panel on Overview and an empty surface here, rather than a sentence about an absence.

import { MergeLine } from "@armada/components";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { useDrafted } from "./drafted";
import { usePanelOpen } from "./panel-open";

/** The panel, where a line was handed over. */
export function MergeLinePanel({ onOpenLink }: { onOpenLink: (address: string) => void }) {
  const { mergeLine } = useDrafted();
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
  bridge,
  onCopied,
  onOpenLink,
}: {
  bridge: BridgeState["bridge"];
  onCopied: (value: string) => void;
  onOpenLink: (address: string) => void;
}) {
  return (
    <Boundary region="Merge line" bridge={bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        <MergeLinePanel onOpenLink={onOpenLink} />
      </div>
    </Boundary>
  );
}

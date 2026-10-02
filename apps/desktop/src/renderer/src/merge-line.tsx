// The merge line, as the window draws it: a panel under Overview's lists, and a rail surface of
// its own. **One panel in both places**, the same rows and the same acts, so nothing on the line
// needs the other view to reach it. They share one fold too.
//
// **Only the mock hands a line over.** Fleet does not serve it yet, so a real Bridge draws neither
// the panel nor the rail row, rather than a sentence about an absence or a row opening nothing.

import { MergeLine } from "@armada/components";
import { Boundary, SURFACE } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import type { Drafted } from "./drafted";
import { useDrafted } from "./drafted";
import { usePanelOpen } from "./panel-open";

/** The surfaces the rail and the palette leave off: the merge line's, until there is a line. */
export function hiddenSurfaces({ mergeLine }: Drafted): readonly string[] {
  return mergeLine === undefined ? [SURFACE.mergeLine] : [];
}

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

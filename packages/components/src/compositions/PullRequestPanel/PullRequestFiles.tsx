import type { PullRequestFile } from "@armada/protocol";

import { SkeletonText } from "../../primitives/Skeleton/Skeleton";
import { UnifiedDiff, type UnifiedDiffProps } from "../UnifiedDiff/UnifiedDiff";
import { DiffSize, FileMark } from "./PullRequestTabs";

/** One changed file's diff, as the host drew it from the forge's patch. A file with no patch has no lines and the forge's sentence as `emptyNote`. */
export type PullRequestPatch = { path: string; diff: UnifiedDiffProps };

/** Every changed file's diff, as the host read them: out, read in the forge's order, or refused in Fleet's words. */
export type PullRequestPatches = { state: "reading" } | { state: "read"; files: readonly PullRequestPatch[] } | { state: "refused"; said: string };

/**
 * The Files tab's body: every file's diff one under the other, as the forge's Files changed view
 * draws them. **Each file is a section named by its path**, the place a press in the file list
 * scrolls to. While the read is out the headers are drawn from the page and only the lines stand in.
 */
export function PatchStack({ rows, patches, target }: { rows: readonly PullRequestFile[]; patches: PullRequestPatches; target: (path: string) => (element: HTMLElement | null) => void }) {
  if (patches.state === "refused")
    return (
      <p className="armada-pr-panel__said" role="alert">
        {patches.said}
      </p>
    );
  const files: readonly { path: string; diff?: UnifiedDiffProps }[] = patches.state === "read" ? patches.files : rows;
  return (
    <div className="armada-pr-patches">
      {files.map((file) => {
        const row = rows.find((one) => one.path === file.path);
        return (
          <section key={file.path} ref={target(file.path)} className="armada-pr-patch" aria-label={file.path}>
            <header className="armada-pr-patch__head">
              {row === undefined ? null : <FileMark status={row.status} />}
              <span className="armada-pr-patch__path mono">{file.path}</span>
              {row === undefined ? null : <DiffSize additions={row.additions} deletions={row.deletions} />}
            </header>
            {file.diff === undefined ? <SkeletonText label={`Reading ${file.path}`} widths={["90%", "75%", "60%"]} /> : <UnifiedDiff {...file.diff} />}
          </section>
        );
      })}
    </div>
  );
}

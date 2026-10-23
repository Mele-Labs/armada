import { FolderGit2 } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

export type RepositoryChipProps = {
  /** The repository's name, as its Manifest names it. */
  name: string;
};

/**
 * A repository as a chip: the repository glyph and the name in mono, on a fill, in the frame the dispatch
 * panel's repository chips wear. **A fact, not a control**: nothing opens from it, and its tooltip names it.
 */
export function RepositoryChip({ name }: RepositoryChipProps) {
  return (
    <Tooltip label="Repository">
      <span className="armada-repository-chip" role="img" aria-label={`Repository ${name}`}>
        <FolderGit2 size={12} strokeWidth={2} aria-hidden />
        <span className="armada-repository-chip__name">{name}</span>
      </span>
    </Tooltip>
  );
}

import { Ban, Folder } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * The folders the owner answered Always allow or Always block for on a session's card: one row each,
 * its mark with the state on hover, its path, and Remove. A removed folder is asked about again.
 */
export type ReachFolderListProps = {
  allowed: readonly string[];
  blocked: readonly string[];
  onRemove: (folder: string) => void;
  /** Nothing can be sent: Fleet is not connected, or a save is under way. */
  disabled?: boolean;
};

const STATE = {
  allowed: { Mark: Folder, said: "Always allowed" },
  blocked: { Mark: Ban, said: "Always blocked" },
} as const;

export function ReachFolderList({ allowed, blocked, onRemove, disabled }: ReachFolderListProps) {
  const rows = [
    ...blocked.map((folder) => ({ folder, state: "blocked" as const })),
    ...allowed.map((folder) => ({ folder, state: "allowed" as const })),
  ];
  if (rows.length === 0) {
    return <p className="armada-reach-folders__none">No folders. Always allow or Always block on a job's card adds one.</p>;
  }
  return (
    <ul className="armada-reach-folders" aria-label="Folders jobs reach">
      {rows.map(({ folder, state }) => {
        const { Mark, said } = STATE[state];
        return (
          <li key={`${state}:${folder}`} className="armada-reach-folders__row">
            <Tooltip label={said}>
              <span className="armada-reach-folders__mark" role="img" aria-label={said}>
                <Mark size={16} />
              </span>
            </Tooltip>
            <code className="armada-reach-folders__path">{folder}</code>
            <Button size="sm" variant="secondary" ground="card" disabled={disabled} aria-label={`Remove ${folder}`} onClick={() => onRemove(folder)}>
              Remove
            </Button>
          </li>
        );
      })}
    </ul>
  );
}

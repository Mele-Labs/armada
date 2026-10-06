import { Folder, GitBranch, KeyRound, Unplug } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ClearSaves } from "./ClearSaves";
import { Line } from "./ConfirmLine";
import { nameOf } from "./tiles";
import type { TileRow } from "./tiles";

/** `nvim (pid 44698)` as the command and the process id, or the whole text where it is not that shape. */
function holderParts(holder: string): { command: string; pid: string | null } {
  const found = /^(.*) \(pid (\d+)\)$/.exec(holder);
  return found === null ? { command: holder, pid: null } : { command: found[1]!, pid: found[2]! };
}

/**
 * What releasing a slot an agent session holds will do, before it is sent.
 * **Uncommitted files are committed to the slot's branch as a WIP commit**, as
 * Clear does for a Job's, and the slot is released; the branch stays. The
 * session's process is not touched, which the third line says.
 */
export function ReleaseConfirm({ row, onSend, onCancel }: { row: TileRow; onSend: () => void; onCancel: () => void }) {
  const { slot } = row;
  if (slot === undefined || slot.held.state !== "session") return null;
  const name = nameOf(row);
  const branch = slot.branch ?? "its branch";
  const files = slot.stranded?.uncommitted ?? [];
  const { command, pid } = holderParts(slot.held.holder);
  return (
    <div className="armada-confirm" role="group" aria-label={`Release ${name}`}>
      <Line
        Glyph={KeyRound}
        said="The agent session that took the slot"
        word={`Held by ${command}, on branch ${branch}`}
        figure={
          pid === null ? undefined : (
            <Tooltip label="Process id">
              <span className="armada-confirm__figure">{`pid ${pid}`}</span>
            </Tooltip>
          )
        }
      />
      <Line Glyph={Unplug} said="The pool's record changes. The process keeps running" word="Takes the checkout from that session" />
      {files.length > 0 ? (
        <ClearSaves branch={branch} files={files} slot={name} path={slot.path} />
      ) : (
        <>
          <Line Glyph={Folder} said="The pool takes the slot back for its next lease" word={`Releases ${name}`} />
          <Line Glyph={GitBranch} said="Nothing on the branch changes" word={`Keeps branch ${branch}`} />
        </>
      )}
      <div className="armada-confirm__choice">
        <Button variant="secondary" size="sm" onClick={onSend}>
          Release
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

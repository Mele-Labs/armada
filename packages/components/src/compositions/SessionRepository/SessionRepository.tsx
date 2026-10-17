import { useState } from "react";
import type { KeyboardEvent } from "react";
import { ChevronDown, FolderGit2, TriangleAlert } from "lucide-react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Popover } from "../../primitives/Popover/Popover";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** A repository a Session can be moved to, by its Manifest id. */
export type SessionRepositoryTarget = { id: string; name: string };

export type SessionRepositoryProps = {
  /** The repository the Session is in now. */
  name: string;
  /** The others Fleet serves. **Absent or empty, the name is plain text.** */
  to?: readonly SessionRepositoryTarget[];
  /** Why none can be picked, said on each. Absent, they can. */
  held?: string;
  /** What the Session leaves behind: its branch where it has one, and its files. Absent, nothing. */
  stays?: { branch?: string };
  /** Moving ends the Session, because it is live. */
  ends?: boolean;
  /** Resolves when the move is done; the host opens the new Session. */
  onMove: (id: string) => Promise<unknown>;
};

/**
 * The repository a Session works in, in its head. **Pressed, it lists the other repositories**, each with the
 * repository glyph, and a pick shows its facts in the same layer: where it goes and what stays behind. Move
 * is the one press that acts. Where there is no other repository the name is text and nothing opens.
 */
export function SessionRepository({ name, to = [], held, stays, ends = false, onMove }: SessionRepositoryProps) {
  if (to.length === 0) {
    return (
      <Tooltip label="Repository">
        <span className="armada-session-repository" data-plain role="img" aria-label={`Repository ${name}`}>
          <FolderGit2 size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-repository__name">{name}</span>
        </span>
      </Tooltip>
    );
  }
  return (
    <Popover
      align="start"
      label="Move to another repository"
      trigger={
        <Tooltip label="Move to another repository">
          <button type="button" className="armada-session-repository" aria-haspopup="dialog" aria-label={`Repository ${name}`}>
            <FolderGit2 size={12} strokeWidth={2} aria-hidden />
            <span className="armada-session-repository__name">{name}</span>
            <ChevronDown size={12} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      }
    >
      {(close) => <MovePanel from={name} to={to} held={held} stays={stays} ends={ends} onMove={onMove} close={close} />}
    </Popover>
  );
}

function MovePanel({ from, to, held, stays, ends, onMove, close }: { from: string; to: readonly SessionRepositoryTarget[]; held: string | undefined; stays: SessionRepositoryProps["stays"]; ends: boolean; onMove: SessionRepositoryProps["onMove"]; close: () => void }) {
  const [picked, setPicked] = useState<SessionRepositoryTarget | undefined>();
  const [moving, setMoving] = useState(false);
  const step = (event: KeyboardEvent<HTMLElement>) => {
    const by = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (by === 0) return;
    event.preventDefault();
    const all = [...event.currentTarget.querySelectorAll<HTMLElement>('[role="menuitem"]')];
    all[(all.indexOf(document.activeElement as HTMLElement) + by + all.length) % all.length]?.focus();
  };
  if (picked !== undefined) {
    const files = stays?.branch === undefined ? <>Its files</> : <><code>{stays.branch}</code> and its files</>;
    return (
      <div className="armada-session-repository__confirm">
        {stays === undefined && !ends ? null : (
          <Alert tone="caution" icon={<TriangleAlert size={16} strokeWidth={2} aria-hidden />}>
            <span className="armada-session-repository__warning">
              {stays === undefined ? null : <span>{files} stay in {from}</span>}
              {ends ? <span>Ends this Job</span> : null}
            </span>
          </Alert>
        )}
        <div className="armada-session-repository__acts">
          <Button
            variant="primary"
            size="sm"
            pending={moving}
            autoFocus
            onClick={() => {
              setMoving(true);
              void onMove(picked.id).finally(() => setMoving(false));
            }}
          >
            {`Move to ${picked.name}`}
          </Button>
          <Button variant="ghost" size="sm" disabled={moving} onClick={close}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }
  return (
    <div className="armada-session-repository__menu" role="menu" aria-label="Repositories" onKeyDown={step}>
      {to.map((one, at) => {
        const item = (
          <button
            key={one.id}
            type="button"
            role="menuitem"
            className="armada-session-repository__item"
            aria-disabled={held !== undefined || undefined}
            autoFocus={at === 0}
            onClick={() => held === undefined && setPicked(one)}
          >
            <FolderGit2 size={12} strokeWidth={2} aria-hidden />
            <span className="armada-session-repository__name">{one.name}</span>
          </button>
        );
        return held === undefined ? item : (
          <Tooltip key={one.id} label={held}>
            {item}
          </Tooltip>
        );
      })}
    </div>
  );
}

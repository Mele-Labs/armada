import { useState } from "react";
import type { ReactNode } from "react";
import { Check, CircleDashed, CircleDot, Eye, ShieldX, SquareTerminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Card } from "../../primitives/Card/Card";
import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Session open, in the glass panel a Job's detail and Overview's panels are
 * (`Card` on the canvas). **The state is one registry mark in the header with
 * its tooltip**, not a colour on the frame: blank is a step not yet started,
 * working is a Drone running, waiting is a Job at review, idle is a turn finished with nothing asked, and failing is a
 * pull request's Checks red. `[conventions.session_state_borrowing]` in
 * `packages/icons/icons/` lends them.
 */
export type SessionState = "blank" | "working" | "waiting" | "failing" | "idle";

const MARK: Record<SessionState, LucideIcon> = {
  blank: CircleDashed,
  working: CircleDot,
  waiting: Eye,
  failing: ShieldX,
  idle: Check,
};

/** The state's one mark, animated while a turn runs, named by its tooltip. */
export function SessionMark({ state, said }: { state: SessionState; said: string }) {
  const Glyph = MARK[state];
  return (
    <Tooltip label={said}>
      <span className="armada-session-mark" role="img" aria-label={said} data-pulsing={state === "working" || undefined}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export type SessionFrameProps = {
  state: SessionState;
  /** What the state's mark says on hover. */
  said: string;
  id: string;
  /** What other sessions call it, drawn in place of `id`. */
  address?: string;
  /** Absent until the first turn has named it. */
  title?: string;
  /**
   * Saves a new name. **Present, the title is edited where it stands**: a press opens it as a field,
   * Enter saves it and Esc, or leaving it, keeps the old one.
   */
  onRename?: (title: string) => void;
  /** Beside the state mark, at the head's trailing edge: the ledger's button where it has folded. */
  actions?: ReactNode;
  /** The ledger, as a panel of its own beside the conversation: a sibling card the height of the first, scrolling on its own. */
  aside?: ReactNode;
  children: ReactNode;
};

/** The title, in place; a field while it is being changed. */
function SessionTitle({ title, onRename }: { title?: string; onRename?: (title: string) => void }) {
  const [draft, setDraft] = useState<string | undefined>(undefined);
  if (onRename === undefined) return <span className="armada-session-frame__title">{title}</span>;
  if (draft === undefined) {
    return (
      <Tooltip label="Rename">
        <button type="button" className="armada-session-frame__title" aria-label={title === undefined ? "Rename" : `${title}, rename`} onClick={() => setDraft(title ?? "")}>
          {title}
        </button>
      </Tooltip>
    );
  }
  const save = () => {
    const next = draft.trim();
    setDraft(undefined);
    if (next !== "" && next !== title) onRename(next);
  };
  return (
    <span className="armada-session-frame__title" data-editing>
      <Input
        aria-label="Session name"
        value={draft}
        autoFocus
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => setDraft(undefined)}
        onKeyDown={(event) => {
          if (event.key === "Enter") save();
          if (event.key === "Escape") {
            event.stopPropagation();
            setDraft(undefined);
          }
        }}
      />
    </span>
  );
}

export function SessionFrame({ state, said, id, address, title, onRename, actions, aside, children }: SessionFrameProps) {
  return (
    <div className="armada-session-panels">
    <Card className="armada-session-frame" role="region" aria-label={`Session ${address ?? id}`}>
      <header className="armada-session-frame__head">
        <SquareTerminal size={16} strokeWidth={2} aria-hidden />
        <span className="armada-session-frame__id">{address ?? id}</span>
        <SessionTitle {...(title === undefined ? {} : { title })} {...(onRename === undefined ? {} : { onRename })} />
        {actions}
        <SessionMark state={state} said={said} />
      </header>
      <div className="armada-session-frame__body">{children}</div>
    </Card>
    {aside === undefined || aside === null ? null : <Card className="armada-session-frame__aside armada-shell__dock">{aside}</Card>}
    </div>
  );
}

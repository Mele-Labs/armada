import { useId, useState } from "react";
import type { KeyboardEvent, MouseEvent as ReactMouseEvent } from "react";
import { BellRing, Check, ChevronRight } from "lucide-react";

import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Prose } from "../../primitives/Prose/Prose";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { NoticeChoice, SessionNotice, SessionThreadRow } from "./SessionThread";

type Message = Extract<SessionThreadRow, { kind: "message" }>;
type Notice = Extract<Message, { from: "session" }>;
type Tool = Extract<SessionThreadRow, { kind: "tool" }>;
export type TurnReply = Extract<Message, { from: "you" | "agent" }> | Tool;

export const NOTICE_CHOICES: { id: NoticeChoice; label: string }[] = [
  { id: "job", label: "For job" },
  { id: "repo", label: "For repo" },
  { id: "hour", label: "For 1 hour" },
];

/** What each notice is called in the menu's header, which reads "Disable <name>". */
export const NOTICE_NAMES: Record<SessionNotice, string> = { behind_base: "slot behind notice" };

/** The folded row's name: the notice's first sentence, up to the first ". " or line break, without its closing full stop. */
export function summaryOf(text: string): string {
  const first = text.trim().split(/\n|\.\s/)[0] ?? "";
  return first.replace(/\.$/, "");
}

/**
 * A turn Fleet started: its notice, the agent's reply and the tool runs between. One dim row; a press opens the notice and the
 * reply under it, quieter than chat, and the same press folds it again.
 */
export function FleetTurn({ notice, reply, handled, onMuteNotice }: { notice: Notice; reply: readonly TurnReply[]; /** The agent answered it, its last words drawn outside the fold or not. */ handled: boolean; onMuteNotice?: (notice: SessionNotice, choice: NoticeChoice) => void }) {
  const [open, setOpen] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number; row: HTMLElement | null } | undefined>(undefined);
  const kind = notice.notice;
  const muting = kind !== undefined && onMuteNotice !== undefined;
  const name = summaryOf(notice.text);
  const body = useId();
  return (
    <li
      className="armada-fleet-turn"
      role="region"
      aria-label="Fleet turn"
      data-open={open}
      {...(!muting
        ? {}
        : {
            onContextMenu: (event: ReactMouseEvent<HTMLLIElement>) => {
              event.preventDefault();
              setMenu({ x: event.clientX, y: event.clientY, row: event.currentTarget.querySelector<HTMLElement>(".armada-fleet-turn__row") });
            },
            onKeyDown: (event: KeyboardEvent<HTMLLIElement>) => {
              if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
              event.preventDefault();
              const box = event.currentTarget.getBoundingClientRect();
              setMenu({ x: box.left + box.width / 2, y: box.bottom, row: event.currentTarget.querySelector<HTMLElement>(".armada-fleet-turn__row") });
            },
          })}
    >
      <button type="button" className="armada-fleet-turn__row" aria-expanded={open} aria-controls={body} onClick={() => setOpen((now) => !now)}>
        <ChevronRight size={12} strokeWidth={2} aria-hidden className="armada-fleet-turn__chevron" />
        <Tooltip label="Fleet notice">
          <span role="img" aria-label="Fleet notice" className="armada-fleet-turn__mark">
            <BellRing size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <Tooltip label={notice.text}>
          <span className="armada-fleet-turn__label">{name}</span>
        </Tooltip>
        {!handled ? null : (
          <Tooltip label="Handled">
            <span role="img" aria-label="Handled" className="armada-fleet-turn__mark">
              <Check size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        )}
      </button>
      {open ? (
        <div id={body} className="armada-fleet-turn__body">
          <p className="armada-fleet-turn__notice">{notice.text}</p>
          {reply.map((one) =>
            one.kind === "tool" ? (
              <p key={one.id} className="armada-fleet-turn__tool">{one.text}</p>
            ) : (
              <div key={one.id} className="armada-fleet-turn__reply">
                <Prose text={one.text} />
              </div>
            ),
          )}
        </div>
      ) : null}
      {menu === undefined || kind === undefined ? null : (
        <DropdownMenu
          triggerLabel="Fleet notice"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={[
            { kind: "label" as const, id: "heading", label: `Disable ${NOTICE_NAMES[kind]}` },
            ...NOTICE_CHOICES.map(({ id, label }) => ({ kind: "item" as const, id, label })),
          ]}
          onOpenChange={(now) => {
            if (now) return;
            setMenu(undefined);
            menu.row?.focus();
          }}
          onSelect={(chosen) => onMuteNotice?.(kind, chosen as NoticeChoice)}
        />
      )}
    </li>
  );
}

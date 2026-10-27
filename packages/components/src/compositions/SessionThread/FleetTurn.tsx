import { useId, useState } from "react";
import type { KeyboardEvent, MouseEvent as ReactMouseEvent } from "react";
import { BellRing, Check, ChevronRight, GraduationCap, Rows3 } from "lucide-react";

import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Prose } from "../../primitives/Prose/Prose";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { NoticeChoice, SessionNotice, SessionThreadRow, TeachPick } from "./SessionThread";

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
export const NOTICE_NAMES: Record<SessionNotice, string> = {
  behind_base: "slot behind notice",
  pr_checks_failed: "checks failed",
  pr_conflicts: "conflicts",
  pr_dropped: "left the queue",
  pr_unmergeable: "unmergeable",
  pr_merged: "merged",
  pr_auto_merge: "auto-merge on",
  pr_queued: "in the queue",
  pr_handed: "pull request handed over",
  slot_free: "slot free",
  slot_kept: "slot kept",
  build_moved: "build moved",
  write_clash: "write clash",
  slot_stranded: "slot stranded",
  main_red: "main red",
  rail_nudge: "rail nudge",
  rail_step: "step instructions",
  usage_trip: "usage notice",
  limit_wake: "limit wake",
  sign_in_wake: "sign-in wake",
  plan_approved: "plan approved",
  auto_land_refused: "auto land refused",
  retro_note: "retro note",
  reply_claim: "reply claim",
  context_handoff: "context handoff",
  walk_failed: "walk failed",
  workflow_changed: "workflow changed",
  delivery_asked: "delivery question",
  delivery_chosen: "delivery chosen",
  watch_saw: "watch",
  taught: "taught",
  issue_lost: "issue lost",
  document_sketch: "document sketch",
  task_marked: "task mark",
};

/**
 * The menu a Fleet notice's row opens: **Teach first, on every notice**, then Disable and its three choices on the one
 * notice that can be turned off.
 */
export function noticeEntries(notice: SessionNotice | undefined, muting: boolean, teaching: boolean): DropdownMenuEntry[] {
  return [
    ...(teaching ? [{ kind: "item" as const, id: "teach", label: "Teach", icon: GraduationCap }] : []),
    ...(teaching && muting ? [{ kind: "separator" as const, id: "rule" }] : []),
    ...(muting && notice !== undefined
      ? [{ kind: "label" as const, id: "heading", label: `Disable ${NOTICE_NAMES[notice]}` }, ...NOTICE_CHOICES.map(({ id, label }) => ({ kind: "item" as const, id, label }))]
      : []),
  ];
}

/** The folded row's name: the notice's first sentence, up to the first ". " or line break, without its closing full stop. */
export function summaryOf(text: string): string {
  const first = text.trim().split(/\n|\.\s/)[0] ?? "";
  return first.replace(/\.$/, "");
}

/**
 * A turn Fleet started: its notice, the agent's reply and the tool runs between. One dim row; a press opens the notice and the
 * reply under it, quieter than chat, and the same press folds it again.
 */
export function FleetTurn({ notice, reply, handled, onMuteNotice, onTeach }: { notice: Notice; reply: readonly TurnReply[]; /** The agent answered it, its last words drawn outside the fold or not. */ handled: boolean; onMuteNotice?: (notice: SessionNotice, choice: NoticeChoice) => void; onTeach?: (pick: TeachPick) => void }) {
  const [open, setOpen] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number; row: HTMLElement | null } | undefined>(undefined);
  const kind = notice.notice;
  const muting = kind !== undefined && onMuteNotice !== undefined;
  const teaching = onTeach !== undefined;
  const name = notice.batch ?? summaryOf(notice.text);
  const body = useId();
  return (
    <li
      className="armada-fleet-turn"
      role="region"
      aria-label="Fleet turn"
      data-open={open}
      {...(!muting && !teaching
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
        {notice.batch === undefined ? null : (
          <Tooltip label="Batched">
            <span role="img" aria-label="Batched" className="armada-fleet-turn__mark">
              <Rows3 size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        )}
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
          {notice.batch === undefined ? (
            <p className="armada-fleet-turn__notice">{notice.text}</p>
          ) : (
            notice.text.split("\n").map((line) => (
              <p key={line} className="armada-fleet-turn__notice">
                {line}
              </p>
            ))
          )}
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
      {menu === undefined || (!muting && !teaching) ? null : (
        <DropdownMenu
          triggerLabel="Fleet notice"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={noticeEntries(kind, muting, teaching)}
          onOpenChange={(now) => {
            if (now) return;
            setMenu(undefined);
            menu.row?.focus();
          }}
          onSelect={(chosen) => {
            if (chosen === "teach") onTeach?.({ kind: "notice", text: notice.text, at: notice.at, ...(notice.notice === undefined ? {} : { ids: [notice.notice] }) });
            else if (kind !== undefined) onMuteNotice?.(kind, chosen as NoticeChoice);
          }}
        />
      )}
    </li>
  );
}

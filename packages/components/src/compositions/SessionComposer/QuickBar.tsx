import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import { ArrowUpToLine, EyeOff, ListPlus, MessageCircleQuestion, MessageSquareQuote, Repeat } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Input } from "../../primitives/Input/Input";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { pinnedFirst } from "../ThreadSettings/thread-look";
import type { QuickAct, QuickPhrase } from "../ThreadSettings/thread-look";

/** Which kind a reply is, by its glyph alone: the tooltip names it. */
const KIND: Record<"asked" | QuickPhrase["kind"], { Glyph: LucideIcon; said: string }> = {
  asked: { Glyph: MessageCircleQuestion, said: "Asked" },
  learnt: { Glyph: Repeat, said: "Learnt" },
  saved: { Glyph: MessageSquareQuote, said: "Saved" },
};

/** A reply's kind, as a glyph named on hover. */
export function QuickKindMark({ kind }: { kind: "asked" | QuickPhrase["kind"] }) {
  const { Glyph, said } = KIND[kind];
  return (
    <Tooltip label={said}>
      <span className="armada-quick__kind" data-kind={kind} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** A pinned line's mark, named on hover. */
export function QuickPinnedMark() {
  return (
    <Tooltip label="Pinned">
      <span className="armada-quick__kind" role="img" aria-label="Pinned">
        <ArrowUpToLine size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * A line's words as a field, selected as it opens. **Enter saves, Esc cancels, leaving it saves.** A line left empty or
 * unchanged is a cancel, so the caller only hears of a real change.
 */
export function QuickEdit({ text, label, onDone }: { text: string; label: string; onDone: (text: string | undefined, by: "key" | "blur") => void }) {
  const field = useRef<HTMLInputElement>(null);
  const settled = useRef(false);
  const [value, setValue] = useState(text);
  useEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);
  const settle = (keep: boolean, by: "key" | "blur") => {
    if (settled.current) return;
    settled.current = true;
    onDone(keep ? value : undefined, by);
  };
  return (
    <span className="armada-quick__edit" data-escape-own>
      <Input
        ref={field}
        aria-label={label}
        autoComplete="off"
        value={value}
        onChange={(event) => setValue(event.currentTarget.value)}
        onBlur={() => settle(true, "blur")}
        onKeyDown={(event) => {
          if (event.key !== "Enter" && event.key !== "Escape") return;
          event.preventDefault();
          event.stopPropagation();
          settle(event.key === "Enter", "key");
        }}
      />
    </span>
  );
}

export type QuickBarProps = {
  /** Answers to the agent's last question, first. */
  asked: readonly string[];
  /** His own lines, learnt and saved, in the order he keeps them. */
  phrases: readonly QuickPhrase[];
  /** A press sends the line as a message. */
  onSend: (text: string) => void;
  /** A ⇧ press puts the line in the box instead. */
  onDrop: (text: string) => void;
  /** Keeps what the box holds as a saved line. Off while the box is empty. */
  onSave: () => void;
  canSave: boolean;
  /** A right press on a chip offers Edit, Pin and Delete, and Save on an answer. Absent, a chip has no menu. */
  onAct?: (act: QuickAct) => void;
  /** Puts the bar away, for this Job or for all Jobs. Absent, the bar has no hide button and no menu on its empty space. */
  onHide?: (scope: "job" | "all") => void;
};

/** The lines the bar offers, in the order ⌘1 to ⌘9 reach them. */
export function quickLines(asked: readonly string[], phrases: readonly QuickPhrase[]): string[] {
  return [...asked, ...pinnedFirst(phrases).map((one) => one.text)];
}

type Chip = { key: string; kind: "asked" | QuickPhrase["kind"]; text: string; id?: string; pinned: boolean };

/**
 * Replies one press away, over the message box: the agent's question's answers, a rule, then his own lines, pinned ones first.
 * **Each chip shows the key that sends it**; a ⇧ press puts it in the box to finish instead. **A right press, the Menu key or
 * ⇧F10 on a chip opens its menu**: Edit, Pin and Delete for his own lines, Save for an answer.
 */
export function QuickBar({ asked, phrases, onSend, onDrop, onSave, canSave, onAct, onHide }: QuickBarProps) {
  const [hideAt, setHideAt] = useState<{ x: number; y: number } | undefined>(undefined);
  const chips: Chip[] = [
    ...asked.map((text) => ({ key: `asked:${text}`, kind: "asked" as const, text, pinned: false })),
    ...pinnedFirst(phrases).map((one) => ({ key: one.id, kind: one.kind, text: one.text, id: one.id, pinned: one.pinned === true })),
  ];
  const kept = new Set(phrases.map((one) => one.text));
  return (
    <div
      className="armada-quick"
      role="group"
      aria-label="Quick replies"
      onContextMenu={
        onHide === undefined
          ? undefined
          : (event) => {
              if ((event.target as Element).closest("button, input") !== null) return;
              event.preventDefault();
              setHideAt({ x: event.clientX, y: event.clientY });
            }
      }
    >
      {chips.flatMap((one, at) => [
        at === asked.length && at > 0 ? <span key="rule" className="armada-quick__rule" aria-hidden /> : null,
        <QuickChip key={one.key} chip={one} at={at} kept={kept.has(one.text)} onSend={onSend} onDrop={onDrop} onAct={onAct} />,
      ])}
      <Tooltip label="Save what is typed as a quick reply">
        <Button type="button" variant="ghost" size="sm" aria-label="Save as quick reply" disabled={!canSave} onClick={onSave}>
          <ListPlus size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
      {onHide === undefined ? null : (
        <>
          <span className="armada-quick__hide">
            <Tooltip label="Hide quick replies">
              <DropdownMenu triggerLabel="Hide quick replies" icon={EyeOff} portal entries={HIDE_ENTRIES} onSelect={(chosen) => onHide(chosen === "all" ? "all" : "job")} />
            </Tooltip>
          </span>
          {hideAt === undefined ? null : (
            <DropdownMenu
              triggerLabel="Hide quick replies"
              at={hideAt}
              open
              entries={HIDE_ENTRIES}
              onOpenChange={(open) => open || setHideAt(undefined)}
              onSelect={(chosen) => onHide(chosen === "all" ? "all" : "job")}
            />
          )}
        </>
      )}
    </div>
  );
}

const HIDE_ENTRIES: DropdownMenuEntry[] = [
  { kind: "item", id: "job", label: "Hide for this Job" },
  { kind: "item", id: "all", label: "Hide for all Jobs" },
];

function QuickChip({ chip, at, kept, onSend, onDrop, onAct }: { chip: Chip; at: number; kept: boolean; onSend: (text: string) => void; onDrop: (text: string) => void; onAct: ((act: QuickAct) => void) | undefined }) {
  const button = useRef<HTMLButtonElement>(null);
  const [menu, setMenu] = useState<{ x: number; y: number } | undefined>(undefined);
  const [editing, setEditing] = useState(false);
  const returning = useRef(false);
  // Leaving the field hands the place back to the chip, so a keyboard is where it was.
  useEffect(() => {
    if (editing || !returning.current) return;
    returning.current = false;
    button.current?.focus();
  }, [editing]);
  if (editing && chip.id !== undefined) {
    const id = chip.id;
    return (
      <QuickEdit
        text={chip.text}
        label={`Edit ${chip.text}`}
        onDone={(text, by) => {
          returning.current = by === "key";
          setEditing(false);
          if (text !== undefined) onAct?.({ act: "edit", id, text });
        }}
      />
    );
  }
  const press = (event: MouseEvent) => (event.shiftKey ? onDrop(chip.text) : onSend(chip.text));
  const openBy = onAct === undefined ? {} : {
    onContextMenu: (event: MouseEvent<HTMLElement>) => {
      event.preventDefault();
      setMenu({ x: event.clientX, y: event.clientY });
    },
    onKeyDown: (event: KeyboardEvent<HTMLElement>) => {
      if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return;
      event.preventDefault();
      const box = event.currentTarget.getBoundingClientRect();
      setMenu({ x: box.left + box.width / 2, y: box.bottom });
    },
  };
  const id = chip.id;
  return (
    <>
      <button ref={button} type="button" className="armada-quick__chip" data-kind={chip.kind} data-pinned={chip.pinned || undefined} aria-haspopup={onAct === undefined ? undefined : "menu"} {...(at < 9 ? { "aria-keyshortcuts": `Meta+${at + 1}` } : {})} onClick={press} {...openBy}>
        <QuickKindMark kind={chip.kind} />
        <span className="armada-quick__text">{chip.text}</span>
        {chip.pinned ? <QuickPinnedMark /> : null}
        {at < 9 ? <Kbd className="armada-quick__key">⌘{at + 1}</Kbd> : null}
      </button>
      {menu === undefined || onAct === undefined ? null : (
        <DropdownMenu
          triggerLabel={`Quick reply ${chip.text}`}
          at={menu}
          open
          entries={
            id === undefined
              ? [{ kind: "item", id: "save", label: "Save", disabled: kept, ...(kept ? { hint: "Already a quick reply" } : {}) }]
              : [
                  { kind: "item", id: "edit", label: "Edit" },
                  { kind: "item", id: chip.pinned ? "unpin" : "pin", label: chip.pinned ? "Unpin" : "Pin" },
                  { kind: "separator", id: "rule" },
                  { kind: "item", id: "delete", label: "Delete", danger: true },
                ]
          }
          onOpenChange={(open) => {
            if (open) return;
            setMenu(undefined);
            button.current?.focus();
          }}
          onSelect={(chosen) => {
            if (chosen === "edit") setEditing(true);
            else if (chosen === "save") onAct({ act: "save", text: chip.text });
            else if (id !== undefined && (chosen === "pin" || chosen === "unpin" || chosen === "delete")) onAct({ act: chosen, id });
          }}
        />
      )}
    </>
  );
}

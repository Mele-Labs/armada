import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent } from "react";
import { BadgeCheck, Ban, MessageSquarePlus, Pencil, Reply } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card, CardFooter } from "../../primitives/Card/Card";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Prose, ProseCommand, ProseRunner } from "../../primitives/Prose/Prose";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { forgetKept, readKept } from "../../keep";
import { isPressed } from "../../keymap";
import { quoteOf } from "../ForkFromSelection/ForkFromSelection";
import type { SessionThreadRow } from "./SessionThread";
import { ThreadKbd, useThreadCard } from "./thread-keys";

export type DraftThreadRow = Extract<SessionThreadRow, { kind: "draft" }>;

/** His answer to a draft, in `answer_waiting`'s words: approved as written, approved with his whole text, or sent back with a note. */
export type DraftAnswer = { choice: 0 } | { fixed: string } | { text: string };

const SETTLED = {
  approved: { said: "Approved", Glyph: BadgeCheck },
  fixed: { said: "Approved with your fixes", Glyph: BadgeCheck },
  sent_back: { said: "Sent back", Glyph: Reply },
  withdrawn: { said: "Withdrawn", Glyph: Ban },
} as const;

/** The words selected in the draft, and where the right press on them was. */
type Opened = { x: number; y: number; words: string };

/** A right press on selected words inside `block`, and what they were. Anywhere else is the press of the row it is in. */
function pressedOn(event: MouseEvent<HTMLElement>, block: HTMLElement): Opened | undefined {
  const selection = window.getSelection();
  if (selection === null || selection.rangeCount === 0 || selection.isCollapsed || !block.contains(selection.anchorNode) || !block.contains(selection.focusNode)) return undefined;
  const words = selection.toString().trim();
  const box = selection.getRangeAt(0).getBoundingClientRect();
  if (words === "" || event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom) return undefined;
  return { x: event.clientX, y: event.clientY, words };
}

/** The draft is text to post, not the agent's commands: a code block in it draws without Run. */
function Text({ text, onContextMenu }: { text: string; onContextMenu?: (event: MouseEvent<HTMLElement>) => void }) {
  return (
    <div className="armada-session-draft__text" {...(onContextMenu === undefined ? {} : { onContextMenu })}>
      <ProseRunner.Provider value={null}>
        <ProseCommand.Provider value={null}>
          <Prose text={text} />
        </ProseCommand.Provider>
      </ProseRunner.Provider>
    </div>
  );
}

/**
 * A draft the agent put to the person with `propose_draft`, approved where it stands
 * (`.claude/decisions/2026-09-29-a-proposal-is-approved-where-it-lands.md`): Approve, Edit and Note in fixed places,
 * and once settled how it ended, with nothing left to press. `keep` names it for the text he types, which a settle forgets.
 * Without `onAnswer` the card is read only. `onWithdraw` adds Withdraw, at the footer's end in read and note mode alike, which in the note
 * takes what he typed as the reason. **A right press on words selected in the text offers Edit and Add note**: Edit opens
 * the box with those words selected, where they stand in the source, and Add note opens the note with them quoted. While it waits it is one
 * of the Job's waiting cards (`thread-keys.tsx`): `⌥↵` Approve, `⌥E` Edit, `⌥R` Note and `⌥X` Withdraw, and `⌘Enter` in a box sends it.
 */
export function DraftRow({ row, keep, onAnswer, onWithdraw }: { row: DraftThreadRow; keep: string; onAnswer?: (rowId: string, answer: DraftAnswer) => void; onWithdraw?: (rowId: string, reason?: string) => void }) {
  const fixedKey = `${keep}:fixed`;
  const noteKey = `${keep}:note`;
  // Text he was typing comes back with its box open.
  const [mode, setMode] = useState<"read" | "edit" | "note">(() => (readKept<string>(fixedKey) !== undefined ? "edit" : readKept<string>(noteKey) !== undefined ? "note" : "read"));
  const [fixed, setFixed] = useState(() => readKept<string>(fixedKey) ?? row.text);
  const [note, setNote] = useState("");
  const [before, setBefore] = useState(false);
  const [menu, setMenu] = useState<Opened>();
  // The words Edit was opened on, selected in the box once it is there.
  const [pick, setPick] = useState<string>();
  const home = useRef<HTMLLIElement>(null);
  const settled = row.settled;
  useEffect(() => {
    if (settled === undefined) return;
    forgetKept(fixedKey);
    forgetKept(noteKey);
  }, [settled, fixedKey, noteKey]);
  const open = settled === undefined && onAnswer !== undefined;
  const editing = open && mode === "edit";
  const noting = open && mode === "note";
  useEffect(() => {
    if (!editing || pick === undefined) return;
    const box = home.current?.querySelector("textarea");
    const at = fixed.indexOf(pick);
    if (box !== null && box !== undefined && at >= 0) box.setSelectionRange(at, at + pick.length);
    setPick(undefined);
  }, [editing, pick, fixed]);
  const cancel = () => {
    forgetKept(fixedKey);
    forgetKept(noteKey);
    setFixed(row.text);
    setNote("");
    setMode("read");
  };
  const state = settled === undefined ? undefined : SETTLED[settled.how];
  // A box opened by its key takes the caret, as one opened by a press does; one already open takes it back.
  const openBox = (to: "edit" | "note") => {
    setMode(to);
    window.requestAnimationFrame(() => home.current?.querySelector<HTMLTextAreaElement>(`textarea[aria-label="${to === "edit" ? "Draft text" : "Note"}"]`)?.focus());
  };
  const approve = () => onAnswer?.(row.id, { choice: 0 });
  const approveFixed = () => onAnswer?.(row.id, fixed === row.text ? { choice: 0 } : { fixed });
  const sendBack = () => onAnswer?.(row.id, { text: note.trim() });
  const withdrawNow = () => onWithdraw?.(row.id, noting && note.trim() !== "" ? note.trim() : undefined);
  const { target, bind } = useThreadCard(
    !open
      ? undefined
      : {
          id: `draft:${row.id}`,
          kind: "Draft",
          title: row.title,
          acts: {
            ...(editing ? (fixed.trim() === "" ? {} : { yes: { verb: "Approve with fixes", run: approveFixed } }) : noting ? {} : { yes: { verb: "Approve", run: approve } }),
            ...(noting ? {} : { change: { verb: "Edit", run: () => openBox("edit") } }),
            ...(editing ? {} : { sendBack: { verb: "Note", run: () => openBox("note") } }),
            ...(onWithdraw === undefined || editing ? {} : { no: { verb: "Withdraw", run: withdrawNow } }),
          },
        },
  );
  // `⌘Enter` in a box sends what it holds, as in every message box.
  const commits = (send: () => void, ready: boolean) => (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!isPressed("send_message", event)) return;
    event.preventDefault();
    if (ready) send();
  };
  const withdraw =
    onWithdraw === undefined ? null : (
      <span className="armada-session-draft__withdraw">
        <Button size="sm" variant="secondary" onClick={withdrawNow}>
          Withdraw
          <ThreadKbd act="no" on={target} />
        </Button>
      </span>
    );
  const choose = (chosen: string) => {
    const words = menu?.words ?? "";
    setMenu(undefined);
    window.getSelection()?.removeAllRanges();
    if (chosen === "edit") {
      setPick(words);
      setMode("edit");
    } else {
      setNote(quoteOf(words));
      setMode("note");
    }
  };
  return (
    <li ref={home} className="armada-session-draft-row">
      <Card flat className="armada-session-draft" role="article" aria-label={`Draft: ${row.title}`} data-settled={settled?.how} {...bind}>
        <div className="armada-session-draft__head">
          <span className="armada-session-thread__eyebrow armada-session-draft__eyebrow">Draft</span>
          <Stamp at={row.at} className="armada-session-draft__at" />
        </div>
        <p className="armada-session-draft__title">{row.title}</p>
        {editing ? (
          <Textarea aria-label="Draft text" rows={8} grow autoFocus keep={fixedKey} value={fixed} onChange={(event) => setFixed(event.currentTarget.value)} onKeyDown={commits(approveFixed, fixed.trim() !== "")} />
        ) : (
          <Text
            text={settled?.how === "fixed" ? (settled.words ?? row.text) : row.text}
            {...(!open ? {} : { onContextMenu: (event: MouseEvent<HTMLElement>) => {
              const there = pressedOn(event, event.currentTarget);
              if (there === undefined) return;
              event.preventDefault();
              setMenu(there);
            } })}
          />
        )}
        {row.before === undefined ? null : (
          <div className="armada-session-draft__before">
            <Button size="sm" variant="ghost" aria-expanded={before} onClick={() => setBefore(!before)}>
              Text it replaces
            </Button>
            {before ? <Text text={row.before} /> : null}
          </div>
        )}
        {noting ? <Textarea aria-label="Note" rows={3} autoFocus keep={noteKey} value={note} onChange={(event) => setNote(event.currentTarget.value)} onKeyDown={commits(sendBack, note.trim() !== "")} /> : null}
        {(settled?.how === "sent_back" || settled?.how === "withdrawn") && settled.words !== undefined ? <blockquote className="armada-session-draft__note">{settled.words}</blockquote> : null}
        {state === undefined ? null : (
          <p className="armada-session-draft__settled" data-how={settled!.how}>
            <Tooltip label={state.said}>
              <span className="armada-session-draft__mark" role="img" aria-label={state.said}>
                <state.Glyph size={16} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <span aria-hidden>{state.said}</span>
          </p>
        )}
        {!open ? null : (
          <CardFooter role="group" aria-label="Answers">
            {editing ? (
              <>
                <Button size="sm" variant="primary" disabled={fixed.trim() === ""} onClick={approveFixed}>
                  Approve with fixes
                  <ThreadKbd act="yes" on={target} />
                </Button>
                <Button size="sm" variant="ghost" onClick={cancel}>
                  Cancel
                </Button>
              </>
            ) : noting ? (
              <>
                <Button size="sm" variant="primary" disabled={note.trim() === ""} onClick={sendBack}>
                  Send back
                </Button>
                <Button size="sm" variant="ghost" onClick={cancel}>
                  Cancel
                </Button>
                {withdraw}
              </>
            ) : (
              <>
                <Button size="sm" variant="primary" onClick={approve}>
                  Approve
                  <ThreadKbd act="yes" on={target} />
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setMode("edit")}>
                  Edit
                  <ThreadKbd act="change" on={target} />
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setMode("note")}>
                  Note
                  <ThreadKbd act="sendBack" on={target} />
                </Button>
                {withdraw}
              </>
            )}
          </CardFooter>
        )}
      </Card>
      {menu === undefined ? null : (
        <DropdownMenu
          triggerLabel="Selection"
          at={{ x: menu.x, y: menu.y }}
          open
          entries={[
            { kind: "item", id: "edit", label: "Edit", icon: Pencil },
            { kind: "item", id: "note", label: "Add note", icon: MessageSquarePlus },
          ]}
          onOpenChange={(now) => {
            if (!now) setMenu(undefined);
          }}
          onSelect={choose}
        />
      )}
    </li>
  );
}

import { useEffect, useRef, useState } from "react";
import type { MouseEvent } from "react";
import { BadgeCheck, Ban, MessageSquarePlus, Pencil, Reply } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card, CardFooter } from "../../primitives/Card/Card";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Prose, ProseCommand, ProseRunner } from "../../primitives/Prose/Prose";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { forgetKept, readKept } from "../../keep";
import { quoteOf } from "../ForkFromSelection/ForkFromSelection";
import type { SessionThreadRow } from "./SessionThread";

export type DraftThreadRow = Extract<SessionThreadRow, { kind: "draft" }>;

/** His answer to a draft, in `answer_waiting`'s words: approved as written, approved with his whole text, or sent back with a note. */
export type DraftAnswer = { choice: 0 } | { fixed: string } | { text: string };

const SCOPE = { kit: "Every repository on this Mac", repository: "This repository" } as const;

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
 * the box with those words selected, where they stand in the source, and Add note opens the note with them quoted.
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
  const proposed = row.workflow;
  const state = settled === undefined ? undefined : proposed !== undefined && settled.how === "withdrawn" ? { said: "Dismissed", Glyph: Ban } : SETTLED[settled.how];
  const withdraw =
    onWithdraw === undefined ? null : (
      <span className="armada-session-draft__withdraw">
        <Button size="sm" variant="secondary" onClick={() => onWithdraw(row.id, noting && note.trim() !== "" ? note.trim() : undefined)}>
          {proposed === undefined ? "Withdraw" : "Dismiss"}
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
      <Card flat className="armada-session-draft" role="article" aria-label={`${proposed === undefined ? "Draft" : "Workflow"}: ${row.title}`} data-settled={settled?.how}>
        <div className="armada-session-draft__head">
          <span className="armada-session-thread__eyebrow armada-session-draft__eyebrow">{proposed === undefined ? "Draft" : "Workflow"}</span>
          <Stamp at={row.at} className="armada-session-draft__at" />
        </div>
        <p className="armada-session-draft__title">{row.title}</p>
        {proposed === undefined ? null : (
          <dl className="armada-session-draft__facts">
            <dt>Name</dt>
            <dd>{proposed.name}</dd>
            <dt>Scope</dt>
            <dd>{SCOPE[proposed.scope]}</dd>
          </dl>
        )}
        {editing ? (
          <Textarea aria-label="Draft text" rows={8} grow autoFocus keep={fixedKey} value={fixed} onChange={(event) => setFixed(event.currentTarget.value)} />
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
        {noting ? <Textarea aria-label="Note" rows={3} autoFocus keep={noteKey} value={note} onChange={(event) => setNote(event.currentTarget.value)} /> : null}
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
                <Button size="sm" variant="primary" disabled={fixed.trim() === ""} onClick={() => onAnswer(row.id, fixed === row.text ? { choice: 0 } : { fixed })}>
                  Approve with fixes
                </Button>
                <Button size="sm" variant="ghost" onClick={cancel}>
                  Cancel
                </Button>
              </>
            ) : noting ? (
              <>
                <Button size="sm" variant="primary" disabled={note.trim() === ""} onClick={() => onAnswer(row.id, { text: note.trim() })}>
                  Send back
                </Button>
                <Button size="sm" variant="ghost" onClick={cancel}>
                  Cancel
                </Button>
                {withdraw}
              </>
            ) : (
              <>
                <Button size="sm" variant="primary" onClick={() => onAnswer(row.id, { choice: 0 })}>
                  Approve
                </Button>
                {proposed !== undefined ? null : (
                  <>
                    <Button size="sm" variant="secondary" onClick={() => setMode("edit")}>
                      Edit
                    </Button>
                    <Button size="sm" variant="secondary" onClick={() => setMode("note")}>
                      Note
                    </Button>
                  </>
                )}
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

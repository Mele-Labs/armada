import { useEffect, useState } from "react";
import { BadgeCheck, Reply } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Card, CardFooter } from "../../primitives/Card/Card";
import { Prose, ProseCommand, ProseRunner } from "../../primitives/Prose/Prose";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { forgetKept, readKept } from "../../keep";
import type { SessionThreadRow } from "./SessionThread";

export type DraftThreadRow = Extract<SessionThreadRow, { kind: "draft" }>;

/** His answer to a draft, in `answer_waiting`'s words: approved as written, approved with his whole text, or sent back with a note. */
export type DraftAnswer = { choice: 0 } | { fixed: string } | { text: string };

const SETTLED = {
  approved: { said: "Approved", Glyph: BadgeCheck },
  fixed: { said: "Approved with your fixes", Glyph: BadgeCheck },
  sent_back: { said: "Sent back", Glyph: Reply },
} as const;

/** The draft is text to post, not the agent's commands: a code block in it draws without Run. */
function Text({ text }: { text: string }) {
  return (
    <div className="armada-session-draft__text">
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
 * Without `onAnswer` the card is read only.
 */
export function DraftRow({ row, keep, onAnswer }: { row: DraftThreadRow; keep: string; onAnswer?: (rowId: string, answer: DraftAnswer) => void }) {
  const fixedKey = `${keep}:fixed`;
  const noteKey = `${keep}:note`;
  // Text he was typing comes back with its box open.
  const [mode, setMode] = useState<"read" | "edit" | "note">(() => (readKept<string>(fixedKey) !== undefined ? "edit" : readKept<string>(noteKey) !== undefined ? "note" : "read"));
  const [fixed, setFixed] = useState(() => readKept<string>(fixedKey) ?? row.text);
  const [note, setNote] = useState("");
  const [before, setBefore] = useState(false);
  const settled = row.settled;
  useEffect(() => {
    if (settled === undefined) return;
    forgetKept(fixedKey);
    forgetKept(noteKey);
  }, [settled, fixedKey, noteKey]);
  const open = settled === undefined && onAnswer !== undefined;
  const editing = open && mode === "edit";
  const noting = open && mode === "note";
  const cancel = () => {
    forgetKept(fixedKey);
    forgetKept(noteKey);
    setFixed(row.text);
    setNote("");
    setMode("read");
  };
  const state = settled === undefined ? undefined : SETTLED[settled.how];
  return (
    <li className="armada-session-draft-row">
      <Card flat className="armada-session-draft" role="article" aria-label={`Draft: ${row.title}`} data-settled={settled?.how}>
        <div className="armada-session-draft__head">
          <span className="armada-session-thread__eyebrow armada-session-draft__eyebrow">Draft</span>
          <Stamp at={row.at} className="armada-session-draft__at" />
        </div>
        <p className="armada-session-draft__title">{row.title}</p>
        {editing ? (
          <Textarea aria-label="Draft text" rows={8} grow autoFocus keep={fixedKey} value={fixed} onChange={(event) => setFixed(event.currentTarget.value)} />
        ) : (
          <Text text={settled?.how === "fixed" ? (settled.words ?? row.text) : row.text} />
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
        {settled?.how === "sent_back" && settled.words !== undefined ? <blockquote className="armada-session-draft__note">{settled.words}</blockquote> : null}
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
              </>
            ) : (
              <>
                <Button size="sm" variant="primary" onClick={() => onAnswer(row.id, { choice: 0 })}>
                  Approve
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setMode("edit")}>
                  Edit
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setMode("note")}>
                  Note
                </Button>
              </>
            )}
          </CardFooter>
        )}
      </Card>
    </li>
  );
}

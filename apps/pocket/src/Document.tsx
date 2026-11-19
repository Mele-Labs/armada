// A Job's document on the phone, opened in the reader (`Reader.tsx`): its words as the page reader Bridge uses draws them
// (`kit/screens/document-page`), which keeps only what its allow list names, the revision picker choosing which kept revision is
// read, and Approve, Ask for changes and Dismiss in the band. A note is made on words he selects and sits under their paragraph;
// Ask for changes sends them with what he types.

import { useCallback, useEffect, useState } from "react";
import { Plus } from "lucide-react";
import type { SketchAnswer } from "./kit/components/compositions/SessionThread/sketch-answer";
import type { BandFacts, Stand } from "./kit/components/compositions/BandCard/BandCard";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";
import { documentAnswerOf } from "./kit/screens/thread-cards";

import { useArtifacts } from "./artifacts";
import { macOf, start, useFleet } from "./fleet";
import { actsOn, attempt, attemptPage, useJob } from "./jobState";
import { PageBody, usePageText, type Head, type Note } from "./PageBody";
import { Reader, type RevisionOf } from "./Reader";
import { go, jobPath } from "./router";
import { documentOf, sessionOfThread } from "./thread";
import { WithKeptStyles } from "./keptStyles";
import { Trouble } from "./ui";

/** What he wrote on each note, put as the words that go back: what he typed, then each note under the words it quotes. */
const putNotes = (typed: string, notes: readonly Note[]): string =>
  [
    typed,
    ...notes
      .filter((one) => one.text.trim() !== "")
      .map((one) => `On "${one.quote}": ${one.text.trim()}`),
  ]
    .filter((part) => part.trim() !== "")
    .join("\n\n");

/** One note, under the paragraph it points at: the words it quotes, and his own. */
function NoteCard({
  note,
  keep,
  onText,
  onDrop,
}: {
  note: Note;
  keep: string;
  onText: (text: string) => void;
  onDrop: () => void;
}) {
  return (
    <div className="pk-note">
      <blockquote className="pk-note__quote">{note.quote}</blockquote>
      <Textarea
        aria-label={`Your note on ${note.quote.slice(0, 40)}`}
        rows={2}
        keep={`${keep}:${note.id}`}
        value={note.text}
        onChange={(event) => onText(event.currentTarget.value)}
      />
      <button type="button" className="pk-bc__act" data-kind="ghost" onClick={onDrop}>
        Remove
      </button>
    </div>
  );
}

export function DocumentView({ origin, id, page }: { origin: string; id: string; page: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read, reread] = useJob(machine, id);
  const [listed] = useArtifacts(machine, id);
  const session = read.thread === undefined ? undefined : sessionOfThread(read.thread);
  const doc = session === undefined ? undefined : documentOf(session, page);
  const artifact = listed.artifacts?.find((one) => one.id === page);
  const [picked, setPicked] = useState<number | undefined>(undefined);
  const [says, setSays] = useState("");
  const [heads, setHeads] = useState<readonly Head[]>([]);
  const [notes, setNotes] = useState<readonly Note[]>([]);
  const [selected, setSelected] = useState<Pick<Note, "quote" | "block"> | undefined>(undefined);
  const count = doc?.revisions.length ?? 0;
  const shown = picked ?? count;
  const text = usePageText(machine, id, doc?.revisions[shown - 1]);
  const title = doc?.title ?? page.slice(page.lastIndexOf("/") + 1);
  const keep = `pocket:${origin}:${id}:document:${page}`;

  const revisions: readonly RevisionOf[] =
    artifact?.revisions ??
    (doc?.revisions ?? []).map((_, at) => ({
      number: at + 1,
      by: "agent" as const,
      at: "",
    }));
  const last = revisions[revisions.length - 1];
  const stand: Stand =
    artifact !== undefined
      ? artifact.state
      : doc?.settled !== undefined
        ? doc.settled.how === "approved"
          ? "approved"
          : "sent_back"
        : doc?.waits === true
          ? "asking"
          : "reading";
  const facts: BandFacts = {
    form: "page",
    title,
    stand,
    words: artifact?.words ?? doc?.settled?.words,
    ...(last === undefined
      ? {}
      : {
          revision: {
            number: last.number,
            said: `r${String(last.number)}`,
          },
        }),
    ...(last?.by === "owner"
      ? {
          yours: `r${String(last.number)} is your edit`,
        }
      : {}),
    forLine: artifact?.for_line,
  };

  const answer = (given: SketchAnswer) => {
    if (machine === undefined || session === undefined) return;
    setSays("");
    void attemptPage(
      () => actsOn(machine, reread).answerDocument(documentAnswerOf(session, page, given)),
      machine,
      reread,
    ).then((refused) => setSays(refused ?? ""));
  };
  const dismiss = () => {
    if (machine === undefined) return;
    setSays("");
    void attempt(
      () =>
        actsOn(machine, reread).dismissWaiting({
          session_id: id,
          item_id: `document:${page}`,
        }),
      machine,
    ).then((refused) => setSays(refused ?? ""));
  };
  const select = useCallback(
    (now: Pick<Note, "quote" | "block"> | undefined) => setSelected(now),
    [],
  );
  const addNote = () => {
    if (selected === undefined) return;
    setNotes((was) => [
      ...was,
      {
        id: `n${String(was.length + 1)}`,
        ...selected,
        text: "",
      },
    ]);
    window.getSelection()?.removeAllRanges();
  };

  return (
    <Reader
      facts={facts}
      keep={keep}
      onBack={() => go(jobPath(origin, id))}
      revisions={revisions}
      shown={shown}
      onPick={(revision) => setPicked(revision === count ? undefined : revision)}
      heads={heads}
      ready={notes.some((one) => one.text.trim() !== "")}
      {...(doc?.waits === true
        ? {
            acts: {
              approve: () =>
                answer({
                  choice: 0,
                }),
              ask: (typed: string) =>
                answer({
                  text: putNotes(typed, notes),
                }),
              dismiss,
            },
          }
        : {})}
      says={says === "" ? undefined : says}
    >
      {text?.kind === "refused" ? <Trouble says={text.says} /> : null}
      {doc === undefined && read.thread !== undefined ? (
        <Trouble says="That Job did not write that page." />
      ) : null}
      {read.thread === undefined ? <Trouble says={read.error ?? ""} /> : null}
      {text?.kind === "text" ? (
        <WithKeptStyles machine={machine}>
          <PageBody
            text={text.text}
            title={title}
            keptStyles={doc?.keptStyles}
            onHeads={setHeads}
            notes={notes}
            onSelect={select}
            renderNote={(note) => (
              <NoteCard
                note={note}
                keep={`${keep}:note`}
                onText={(typed) =>
                  setNotes((was) =>
                    was.map((one) =>
                      one.id === note.id
                        ? {
                            ...one,
                            text: typed,
                          }
                        : one,
                    ),
                  )
                }
                onDrop={() => setNotes((was) => was.filter((one) => one.id !== note.id))}
              />
            )}
          />
        </WithKeptStyles>
      ) : null}
      {selected === undefined || doc?.waits !== true ? null : (
        <div className="pk-note-add">
          <Tooltip label="Add a note under this paragraph">
            <button type="button" className="pk-bc__act" data-kind="secondary" onClick={addNote}>
              <Plus size={14} aria-hidden="true" />
              Add note
            </button>
          </Tooltip>
        </div>
      )}
    </Reader>
  );
}

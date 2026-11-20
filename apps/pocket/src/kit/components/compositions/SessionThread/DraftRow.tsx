import type { TeachPick, SessionThreadRow } from "./SessionThread";
import { BandCard, Well, type BandFacts, type Stand } from "../BandCard/BandCard";
import { ProseRunner } from "../../primitives/Prose/RunnableBlock";
import { ProseCommand } from "../../primitives/Prose/SlashCommand";
import { Prose } from "../../primitives/Prose/Prose";

export type DraftThreadRow = Extract<
  SessionThreadRow,
  {
    kind: "draft";
  }
>;

/** His answer to a draft, in `answer_waiting`'s words: approved as written, approved with his whole text, or sent back with a note. */
export type DraftAnswer =
  | {
      choice: 0;
    }
  | {
      fixed: string;
    }
  | {
      text: string;
    };

/** The draft is text to post, not the agent's commands: a code block in it draws without Run. */
export function DraftText({ text }: { text: string }) {
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
 * A draft the agent put to the person with `propose_draft`, as the Band card native Bridge draws (`BandCard`): a header band
 * filled with where it stands, its text in a well, and its answers in a footer. Once settled it is one line saying how it ended.
 * Without `onAnswer` the card is for reading. `keep` names it for the note he types, which a settle forgets.
 */
export function DraftRow({
  row,
  keep,
  onAnswer,
  onWithdraw,
}: {
  row: DraftThreadRow;
  keep: string;
  onAnswer?: (rowId: string, answer: DraftAnswer) => void;
  onWithdraw?: (rowId: string, reason?: string) => void;
  onTeach?: (pick: TeachPick) => void;
  headed?: boolean;
}) {
  const settled = row.settled;
  const open = settled === undefined && onAnswer !== undefined;
  const stand: Stand =
    settled === undefined
      ? open
        ? "asking"
        : "reading"
      : settled.how === "sent_back"
        ? "sent_back"
        : settled.how === "withdrawn"
          ? row.workflow !== undefined
            ? "dismissed"
            : "withdrawn"
          : "approved";
  const facts: BandFacts = {
    form: "draft",
    title: row.title,
    stand,
    words:
      settled?.how === "fixed" ? "Approved with your fixes" : (settled?.words ?? row.facts?.words),
    ...(row.facts?.revision === undefined
      ? {}
      : {
          revision: row.facts.revision,
        }),
    yours: row.facts?.yours,
    forLine: row.facts?.forLine,
  };
  return (
    <BandCard
      facts={facts}
      keep={keep}
      well={
        <Well>
          <DraftText text={settled?.how === "fixed" ? (settled.words ?? row.text) : row.text} />
        </Well>
      }
      acts={
        open
          ? {
              approve: () =>
                onAnswer?.(row.id, {
                  choice: 0,
                }),
              ask: (note: string) =>
                onAnswer?.(row.id, {
                  text: note,
                }),
              ...(onWithdraw === undefined
                ? {}
                : {
                    dismiss: () => onWithdraw(row.id),
                  }),
            }
          : {}
      }
      asked={open}
    />
  );
}

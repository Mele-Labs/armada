import type { SessionThreadRow } from "./SessionThread";
import type { SketchAnswer } from "./sketch-answer";
import type { ReactNode } from "react";
import { BandCard, Well, type BandFacts, type Stand } from "../BandCard/BandCard";

/**
 * A page the Job wrote as its document, where in the thread Fleet kept its newest revision, as the Band card native Bridge draws
 * (`BandCard`): a header band filled with where it stands, the page's first screen in a well, and its answers in a footer. **A page
 * is for reading unless it asks**: only a revision Fleet put to the person (`asked`) holds Approve and Ask for changes, and once
 * answered it is one line saying how it ended. A press on the band or the well opens the page over the Job. `screen` is the
 * first screen, which the host reads: the card draws none of its own.
 */
export function DocumentRow({
  row,
  keep,
  screen,
  onOpen,
  onAnswer,
  onWithdraw,
}: {
  row: Extract<
    SessionThreadRow,
    {
      kind: "document";
    }
  >;
  keep?: string;
  screen?: ReactNode;
  onOpen?: (page: string) => void;
  onAnswer?: (page: string, answer: SketchAnswer) => void;
  onWithdraw?: (page: string) => void;
}) {
  const settled = row.settled;
  const open = row.asked === true && settled === undefined && onAnswer !== undefined;
  const stand: Stand =
    settled !== undefined
      ? settled.how === "approved"
        ? "approved"
        : "sent_back"
      : row.asked === true
        ? "asking"
        : "reading";
  const facts: BandFacts = {
    form: "page",
    title: row.title,
    stand,
    words: settled?.words ?? row.facts?.words,
    revision: row.facts?.revision ?? {
      number: row.revision,
      said: `r${String(row.revision)}`,
    },
    yours: row.facts?.yours,
    forLine: row.facts?.forLine,
  };
  return (
    <BandCard
      facts={facts}
      keep={keep ?? `document:${row.id}`}
      {...(screen === undefined
        ? {}
        : {
            well: <Well>{screen}</Well>,
          })}
      acts={{
        ...(onOpen === undefined
          ? {}
          : {
              open: () => onOpen(row.page),
            }),
        ...(open
          ? {
              approve: () =>
                onAnswer?.(row.page, {
                  choice: 0,
                }),
              ask: (note: string) =>
                onAnswer?.(row.page, {
                  text: note,
                }),
              ...(onWithdraw === undefined
                ? {}
                : {
                    dismiss: () => onWithdraw(row.page),
                  }),
            }
          : {}),
      }}
      asked={open}
    />
  );
}

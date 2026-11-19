// What Bridge says when it refuses something, and the words on the answers to a call that is
// waiting on a person.
//
// Beside `App.tsx` rather than inside it because it is copy: every sentence
// here is read by a person and none of it is wiring. **No status label is
// among them** — those come from the generated vocabulary, which is what stops
// a second one being typed into a component.

import type { CallAnswer } from "../../wire/index";

/**
 * What each answer to a held call commits to. #1389.
 *
 * Its own record beside `COMMAND_ANSWER`, because the two sets are different
 * sets: there is no job to allow for, and `allow_and_remember` writes a rule
 * into the person's own settings rather than into fleet's table.
 */
export const CALL_ANSWER: Record<
  CallAnswer,
  {
    label: string;
    means: string;
  }
> = {
  allow_once: {
    label: "Allow once",
    means: "Runs it now. Nothing is written down, and the same call asks again next time.",
  },
  allow_and_remember: {
    label: "Allow and remember",
    means:
      "Runs it now, and the rule goes into this repository's own personal agent settings " +
      "— so your terminal stops asking too.",
  },
  refuse: {
    label: "Refuse",
    means: "The job is told no, and says what it could not do.",
  },
  always_allow: {
    label: "Always allow",
    means:
      "Runs it now. Every job reads and writes in the folder chosen, and under it, with no card. Settings, Features lists it.",
  },
  always_block: {
    label: "Always block",
    means:
      "Not run. Every job is refused the folder chosen, and under it, with no card. Settings, Features lists it.",
  },
  allow_for_session: {
    label: "Always allow for this job",
    means:
      "Runs it now. This job reads and writes in the folder chosen, and under it, with no card, until it is closed. Settings does not list it.",
  },
  allow_like_this_for_session: {
    label: "Allow calls like this for this job",
    means:
      "Runs it, and every call of this kind until the job is closed. Settings does not list it.",
  },
};

/** The once answers on a card over a session reaching past its slots, in the owner's words for them. */
const REACH_ONCE: Partial<
  Record<
    CallAnswer,
    {
      label: string;
      means: string;
    }
  >
> = {
  allow_once: {
    label: "Allow once",
    means: "Runs it now. The next call outside its slots asks again.",
  },
  refuse: {
    label: "Reject once",
    means: "Not run. The job is told no, and the next call asks again.",
  },
};

/** [`offeredOf`] for a card that names the path it reaches: Refuse reads as Reject once. */
export function reachOfferedOf(offers: readonly CallAnswer[]): OfferedAnswer[] {
  return offeredOf(offers).map((one) => ({
    ...one,
    ...REACH_ONCE[one.offer],
  }));
}

/** What a person may answer about one held call, worded, in fleet's own order. */
export function offeredOf(offers: readonly CallAnswer[]): OfferedAnswer[] {
  const known: Partial<
    Record<
      string,
      {
        label: string;
        means: string;
      }
    >
  > = CALL_ANSWER;
  // An answer from a fleet ahead of this build is left out, `offeredOf`'s rule.
  return offers.flatMap((offer) => {
    const said = known[offer];
    return said === undefined
      ? []
      : [
          {
            offer,
            ...said,
          },
        ];
  });
}

/** One answer, worded. No rule picker: fleet already derived the one rule. */
export type OfferedAnswer = {
  offer: CallAnswer;
  label: string;
  means: string;
};

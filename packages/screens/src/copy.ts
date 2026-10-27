// What Bridge says when it refuses something, and the words on the answers to a call that is
// waiting on a person.
//
// Beside `App.tsx` rather than inside it because it is copy: every sentence
// here is read by a person and none of it is wiring. **No status label is
// among them** — those come from the generated vocabulary, which is what stops
// a second one being typed into a component.

import type { HelmCallAnswer, Outcome } from "@armada/protocol";
import { NOTHING_SERVED, servesNothing } from "./locate-reads";

/** What a refusal says. Every one names what happened and what to do. */
export function said(outcome: Outcome): string {
  if (outcome.ok) return "";
  switch (outcome.why) {
    case "empty_brief":
      return "A message needs some text. Nothing was sent.";
    case "no_manifest":
      return "This needs a Manifest Fleet holds. Nothing was sent.";
    case "not_connected":
      return "Fleet is not connected. Nothing was sent.";
    case "not_understood":
      return "Bridge could not read that request. Nothing was sent.";
    case "not_set_up":
      return "This repository has no Manifest yet. Set it up first. Nothing was sent.";
    case "already_answering_lesson":
      return "That answer is already in flight. It was not sent twice.";
    case "already_setting":
      return "A change to the settings is already in flight. It was not sent twice.";
    case "nothing_held":
      return "Nothing is held, so there is nothing to capture. Point at something on the page first.";
    case "run_ended":
      return (
        "This run stopped serving while the note was being written, so nothing was captured — " +
        "what is on the port now may not be the run. Start the server again and capture there."
      );
    case "no_owner":
      return "Nothing opened this window, so there is nobody to tell it was approved.";
    case "refused":
      // Drawn as a failure notice above, with everything it carries — except Fleet serving nothing, which is no fault.
      return servesNothing(outcome) ? `${NOTHING_SERVED.title}. ${NOTHING_SERVED.next}` : "";
    case "transport":
      // **The fallback and not the rendering.** `App.tsx` draws this through
      // `transportFailure`, with a code, the route, the wait and something to
      // copy — this is what is left for a surface that has only a sentence to
      // put it in, so it names the route rather than repeating the machine's
      // words on their own. "Fleet did not answer" is the same line whichever
      // route it was about, and that was the whole complaint.
      return `Fleet did not answer ${outcome.fault.method} ${outcome.fault.path}: ${outcome.detail}`;
  }
}

/**
 * What each answer to a held **helm** call commits to. #1389.
 *
 * Its own record beside `COMMAND_ANSWER`, because the two sets are different
 * sets: there is no job to allow for, and `allow_and_remember` writes a rule
 * into the person's own settings rather than into fleet's table.
 */
export const HELM_CALL_ANSWER: Record<HelmCallAnswer, { label: string; means: string }> = {
  allow_once: {
    label: "Allow once",
    means: "Helm runs it now. Nothing is written down, and the same call asks again next time.",
  },
  allow_and_remember: {
    label: "Allow and remember",
    means:
      "Helm runs it now, and the rule goes into this repository's own personal agent settings " +
      "— so your terminal stops asking too.",
  },
  refuse: {
    label: "Refuse",
    means: "Helm is told no, and says what it could not do.",
  },
  always_allow: {
    label: "Always allow",
    means: "Runs it now. Every job reads and writes in the folder chosen, and under it, with no card. Settings, Features lists it.",
  },
  always_block: {
    label: "Always block",
    means: "Not run. Every job is refused the folder chosen, and under it, with no card. Settings, Features lists it.",
  },
  allow_for_session: {
    label: "Always allow for this job",
    means: "Runs it now. This job reads and writes in the folder chosen, and under it, with no card, until it is closed. Settings does not list it.",
  },
  allow_like_this_for_session: {
    label: "Allow calls like this for this job",
    means: "Runs it, and every call of this kind until the job is closed. Settings does not list it.",
  },
};

/** The once answers on a card over a session reaching past its slots, in the owner's words for them. */
const REACH_ONCE: Partial<Record<HelmCallAnswer, { label: string; means: string }>> = {
  allow_once: { label: "Allow once", means: "Runs it now. The next call outside its slots asks again." },
  refuse: { label: "Reject once", means: "Not run. The job is told no, and the next call asks again." },
};

/** [`helmOfferedOf`] for a card that names the path it reaches: Refuse reads as Reject once. */
export function reachOfferedOf(offers: readonly HelmCallAnswer[]): OfferedHelmAnswer[] {
  return helmOfferedOf(offers).map((one) => ({ ...one, ...REACH_ONCE[one.offer] }));
}

/** What a person may answer about one helm call, worded, in fleet's own order. */
export function helmOfferedOf(offers: readonly HelmCallAnswer[]): OfferedHelmAnswer[] {
  const known: Partial<Record<string, { label: string; means: string }>> = HELM_CALL_ANSWER;
  // An answer from a fleet ahead of this build is left out, `offeredOf`'s rule.
  return offers.flatMap((offer) => {
    const said = known[offer];
    return said === undefined ? [] : [{ offer, ...said }];
  });
}

/** One helm answer, worded. No rule picker: fleet already derived the one rule. */
export type OfferedHelmAnswer = { offer: HelmCallAnswer; label: string; means: string };

// What the owner can do from the flight deck that Fleet answers: Reply, Stop, Merge and Open.
//
// **Each act is Fleet's or it is not offered.** Nothing is done locally and then said to have happened: a pull request is
// not marked landed, a Session is not marked stopped. Where Fleet has no way to do an act for a subject, `can` says so and
// the surface leaves the press off rather than drawing it dead. A Session's merge is the by-number act on its pull
// request. #2301.

import { useMemo } from "react";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import { openPullRequestInPanel } from "../../shared-panel/store";
import type { CockpitProps } from "./props";
import type { Subject } from "./ship";

type Hosts = Pick<CockpitProps, "onOpenSession">;

export type Acts = {
  reply: (subject: Subject, text: string) => void;
  stop: (subject: Subject) => void;
  merge: (subject: Subject) => void;
  open: (subject: Subject) => void;
  /** Whether Fleet has the act for this one. A press that is not offered is left off. */
  can: {
    stop: (subject: Subject) => boolean;
    merge: (subject: Subject) => boolean;
  };
};

/** What a ring's button, a Session's flight control and the Plan panel ask a Session. A plan ticks only where the task list names its tasks word for word. */
export const ASK_FOR_UPDATE = "Bring your task list up to date with TaskUpdate, using the plan's task text word for word if you recorded a plan, then say what is done, what is running now, what is next, and anything blocked.";

/**
 * Asks a Session for an update, through the route its composer sends with. Resolves whether Fleet took it: a refusal is worded by the
 * Sessions draft itself, as for any message, and false is also what an absent draft answers.
 */
export async function askForUpdate(draft: SessionsDraft | undefined, sessionId: string): Promise<boolean> {
  if (draft === undefined) return false;
  return (await draft.send(sessionId, { text: ASK_FOR_UPDATE, files: [], sketches: [], tags: [] })) !== false;
}

/** What a Later row's Remind sends: the sentence the agent said it would act on, quoted back, and the three ways to settle it. */
export const remindText = (sentence: string): string => `You said: "${sentence}" Do it, file it, or drop it with a reason.`;

/** Reminds a Session of a follow-up it kept, through the route its composer sends with. Resolves whether Fleet took it, as `askForUpdate` does. */
export async function remindLater(draft: SessionsDraft | undefined, sessionId: string, sentence: string): Promise<boolean> {
  if (draft === undefined) return false;
  return (await draft.send(sessionId, { text: remindText(sentence), files: [], sketches: [], tags: [] })) !== false;
}

/** The acts over what they are made of. Pure, so what each does and does not reach is held by a test. */
export function actsOf(draft: SessionsDraft | undefined, hosts: Hosts): Acts {
  const { onOpenSession } = hosts;
  const can: Acts["can"] = {
    stop: (subject) => subject.kind === "session" && draft?.stop !== undefined,
    merge: (subject) => subject.pr !== undefined && subject.kind === "session" && draft !== undefined,
  };
  return {
    reply: (subject, text) => {
      void draft?.send(subject.id, { text, files: [], sketches: [], tags: [] });
    },
    stop: (subject) => {
      if (subject.kind === "session") draft?.stop?.(subject.id);
    },
    merge: (subject) => {
      const pr = subject.pr;
      if (pr === undefined) return;
      if (subject.kind === "session") void draft?.act(subject.id, pr.number, "merge");
    },
    open: (subject) => {
      if (subject.kind === "session") onOpenSession(subject.id);
      else if (subject.pr !== undefined) openPullRequestInPanel(subject.pr);
    },
    can,
  };
}

export function useShipActs(props: Hosts): Acts {
  const draft = useSessionsDraft();
  const { onOpenSession } = props;
  return useMemo(() => actsOf(draft, { onOpenSession }), [draft, onOpenSession]);
}

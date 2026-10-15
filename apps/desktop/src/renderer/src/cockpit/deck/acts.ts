// What the owner can do from the flight deck that Fleet answers: Reply, Stop, Fix and Merge, and Open.
//
// **Each act is Fleet's or it is not offered.** Nothing is done locally and then said to have happened: a pull request is
// not marked landed, a Session is not marked stopped, main is not marked fixed. Where Fleet has no way to do an act for a
// subject, `can` says so and the surface leaves the press off rather than drawing it dead. A Session's merge is the
// by-number act on its pull request; a Job's merge is the command the review gate already sends, handed in by the page
// that owns commands, because the Cockpit's hosts do not carry it. #2301.

import { useMemo } from "react";
import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import { useSessionsDraft } from "../../sessions-draft";
import type { CockpitProps } from "./props";
import type { Main, Subject } from "./ship";

type Hosts = Pick<CockpitProps, "onKill" | "onFix" | "onOpen" | "onOpenSession" | "onOpenLink">;

/** What a page that owns commands can add: the Job's merge. Absent, a Job's Merge is not offered. */
export type Extra = { onMergeJob?: ((jobId: string) => void | Promise<void>) | undefined };

export type Acts = {
  reply: (subject: Subject, text: string) => void;
  stop: (subject: Subject) => void;
  merge: (subject: Subject) => void;
  fix: (main: Main) => void;
  open: (subject: Subject) => void;
  /** Whether Fleet has the act for this one. A press that is not offered is left off. */
  can: {
    stop: (subject: Subject) => boolean;
    merge: (subject: Subject) => boolean;
    fix: () => boolean;
  };
};

/** The acts over what they are made of. Pure, so what each does and does not reach is held by a test. */
export function actsOf(draft: SessionsDraft | undefined, hosts: Hosts & Extra): Acts {
  const { onKill, onFix, onOpen, onOpenSession, onOpenLink, onMergeJob } = hosts;
  const can: Acts["can"] = {
    stop: (subject) => (subject.kind === "session" ? draft?.stop !== undefined : subject.kind === "job" && onKill !== undefined),
    merge: (subject) => subject.pr !== undefined && (subject.kind === "session" ? draft !== undefined : subject.kind === "job" && onMergeJob !== undefined),
    fix: () => onFix !== undefined,
  };
  return {
    reply: (subject, text) => {
      void draft?.send(subject.id, { text, files: [], sketches: [], tags: [] });
    },
    stop: (subject) => {
      if (subject.kind === "session") draft?.stop?.(subject.id);
      else if (subject.kind === "job") onKill?.(subject.id);
    },
    merge: (subject) => {
      const pr = subject.pr;
      if (pr === undefined) return;
      if (subject.kind === "session") void draft?.act(subject.id, pr.number, "merge");
      else if (subject.kind === "job") void onMergeJob?.(subject.id);
    },
    fix: (main) => {
      onFix?.({ root: main.root });
    },
    open: (subject) => {
      if (subject.kind === "job") onOpen(subject.id);
      else if (subject.kind === "session") onOpenSession(subject.id);
      else if (subject.pr !== undefined) onOpenLink(subject.pr.url);
    },
    can,
  };
}

/** What Ask for update says to a Session. One text for every place that asks: the deck's flight control and the build ring's button. */
export const ASK_FOR_UPDATE = "Progress update, please: what is done, what is running now, what is next, and anything blocked.";

/** Asks a Session for an update through the route its composer sends by. Resolves to whether the message arrived; a refusal is already toasted. */
export async function askForUpdate(draft: SessionsDraft, sessionId: string): Promise<boolean> {
  return (await draft.send(sessionId, { text: ASK_FOR_UPDATE, files: [], sketches: [], tags: [] })) !== false;
}

export function useShipActs(props: Hosts, extra: Extra = {}): Acts {
  const draft = useSessionsDraft();
  const { onKill, onFix, onOpen, onOpenSession, onOpenLink } = props;
  const { onMergeJob } = extra;
  return useMemo(() => actsOf(draft, { onKill, onFix, onOpen, onOpenSession, onOpenLink, onMergeJob }), [draft, onKill, onFix, onOpen, onOpenSession, onOpenLink, onMergeJob]);
}

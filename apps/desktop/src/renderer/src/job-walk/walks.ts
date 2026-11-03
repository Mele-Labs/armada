// The walks a Job owes him, read from what the Job showed in a window: each `window` artifact that carries a walk. The tab of each is `sync.tsx`'s, and the real wiring reads the
// same attachments, so nothing here knows the mock.

import { attachmentsOf, type Session } from "@armada/screens/src/draft/sessions";

import { isApproved } from "./store";

export type JobWalk = {
  /** The page's address, which names the walk and is kept on its tab. */
  id: string;
  /** The page's name, which its tab reads. */
  title: string;
  /** What the Job changed and what to look at, in its words. Absent: it said nothing. */
  brief?: string;
  takes: readonly string[];
  steps: readonly {
    say: string;
    target: string;
  }[];
};

/** Every walk the Job showed. */
export const walksOf = (session: Session): JobWalk[] =>
  attachmentsOf(session, "artifact").flatMap((one) =>
    one.form !== "window" || one.walk === undefined
      ? []
      : [
          {
            id: one.id,
            title: one.title,
            ...(one.walk.brief === undefined
              ? {}
              : {
                  brief: one.walk.brief,
                }),
            takes: one.walk.takes ?? [],
            steps: one.walk.steps,
          },
        ],
  );

/** The walks he has not approved. */
export const owedWalks = (session: Session): JobWalk[] =>
  walksOf(session).filter((one) => !isApproved(session.id, one.id));

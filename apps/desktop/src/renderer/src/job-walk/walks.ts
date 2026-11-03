// The walks a Job owes him, read from what the Job showed in a window: each `window` artifact that carries a walk. The tab of each is `sync.tsx`'s, and the real wiring reads the
// same attachments, so nothing here knows the mock.

import { attachmentsOf, owesOf, type Session } from "@armada/screens/src/draft/sessions";

import { isApproved } from "./store";

export type JobWalk = {
  /** The page's address, which names the walk and is kept on its tab. */
  id: string;
  /** The page's name, which its tab reads. */
  title: string;
  /** What the Job changed and what to look at, in its words. Absent: it said nothing. */
  brief?: string;
  takes: readonly string[];
  /** The mock's steps point at a place on its page by name; a real walk's page is another program's, so its steps carry their words only. */
  steps: readonly {
    say: string;
    target?: string;
  }[];
  /** Read from the mock's field, whose walk is owed until the mock approves it. A real walk is owed while Fleet says it waits. */
  mock: boolean;
};

/** A page the Job gave no name, whose title Fleet says is its address: a mock's walk, else the end of its path, else its host. */
const nameOfPage = (address: string): string => {
  try {
    const url = new URL(address);
    return (
      url.searchParams.get("walk") ?? url.pathname.split("/").filter(Boolean).at(-1) ?? url.host
    );
  } catch {
    return address;
  }
};

/** Every walk the Job showed. */
export const walksOf = (session: Session): JobWalk[] =>
  attachmentsOf(session, "artifact").flatMap((one): JobWalk[] => {
    if (one.form !== "window") return [];
    const read = one.walk ?? one.window;
    if (read === undefined) return [];
    return [
      {
        id: one.id,
        title: one.title === one.id ? nameOfPage(one.id) : one.title,
        ...(read.brief === undefined
          ? {}
          : {
              brief: read.brief,
            }),
        takes: read.takes ?? [],
        steps: read.steps,
        mock: one.walk !== undefined,
      },
    ];
  });

/**
 * The walks he is owed. A real walk is owed while the Job's waiting item for it stands, which is what Needs you reads: Fleet drops it when he approves the page or the Job withdraws it.
 * The mock has no Fleet, so it keeps its own approval.
 */
export const owedWalks = (session: Session): JobWalk[] => {
  const waiting = new Set(
    owesOf(session).flatMap((one) =>
      one.source === "walk" && one.act?.kind === "walk"
        ? [
            one.act.target,
          ]
        : [],
    ),
  );
  return walksOf(session).filter((one) =>
    one.mock ? !isApproved(session.id, one.id) : waiting.has(one.id),
  );
};

// A Job working an issue: as its live state's glyph, the one the Cockpit's list draws, named by a tooltip that says which
// Job; or as the shared Job card, where there is room for one.

import { JobCard, SessionMark } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { peekOfSession } from "../cockpit/deck/ship";
import { stateOf } from "../session-state";
import { useSessions } from "../sessions-draft";

/** The Jobs Fleet's ledger says are working this issue, as the window holds them now. A Job no longer held is left out. */
export function useHolders(ids: readonly string[] | undefined): Session[] {
  const sessions = useSessions();
  return (ids ?? []).flatMap((id) => sessions.filter((one) => one.id === id));
}

export const jobTitle = (session: Session): string => session.title ?? session.address ?? session.id;

export function JobMark({ session }: { session: Session }) {
  const { state, said } = stateOf(session);
  return <SessionMark state={state} said={`${jobTitle(session)}: ${said}`} />;
}

/** The Job as the shared card: its state, title, the last line it said and its age. The issue's repository is the Job's, so no chip. */
export function IssueJobCard({ session, onOpen }: { session: Session; onOpen: (id: string) => void }) {
  const { state, said } = stateOf(session);
  const says = peekOfSession(session).at(-1);
  return <JobCard title={jobTitle(session)} state={state} said={said} {...(says === undefined ? {} : { says })} {...(session.lastTurnAt === undefined ? {} : { at: session.lastTurnAt })} onOpen={() => onOpen(session.id)} />;
}

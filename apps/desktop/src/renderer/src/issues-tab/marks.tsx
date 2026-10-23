// A Job working an issue, as its live state's glyph: the one the Cockpit's list draws, named by a tooltip that says which Job.

import { SessionMark } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

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

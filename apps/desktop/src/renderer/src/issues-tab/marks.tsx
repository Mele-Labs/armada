// A Job working an issue: as its live state's glyph, the one the Cockpit's list draws, named by a tooltip that says which
// Job; or as the shared Job card, where there is room for one.

import { Bug, CircleDot, Save } from "lucide-react";
import { JobCard, SessionMark, Tooltip, jobToneOf, type JobTone } from "@armada/components";
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

const LOUD: readonly JobTone[] = ["bad", "ask", "live", "calm"];

/** The hue an issue's row wears: the loudest state among the Jobs working it. None working, none. */
export function toneOfHolders(holders: readonly Session[]): JobTone | undefined {
  const tones = holders.map((one) => jobToneOf(stateOf(one).state));
  return LOUD.find((tone) => tones.includes(tone));
}

/** A report not yet opened: a quiet ring, drawn in the muted ink. */
export function UnreadMark() {
  return (
    <Tooltip label="Unread">
      <span className="issues-mark" role="img" aria-label="Unread">
        <CircleDot size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** A report he kept. */
export function KeptMark() {
  return (
    <Tooltip label="Kept">
      <span className="issues-mark" role="img" aria-label="Kept">
        <Save size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** On a Job's row in the Cockpit: this Job reported a bug. A press opens the report on the Issues tab. */
export function ReportedMark({ onOpen }: { onOpen: () => void }) {
  return (
    <Tooltip label="Reported a bug" asChild>
      <button type="button" className="deck-row__reported-mark" aria-label="Reported a bug" onClick={onOpen}>
        <Bug size={12} strokeWidth={2} aria-hidden />
      </button>
    </Tooltip>
  );
}

// The standing conditions, above whatever surface is open.
//
// **A band, not a screen.** Everything here is true regardless of which surface
// a person is on, stays on screen until they put it away, and leaves the
// surface working beneath it — which is the design contract's own definition of
// a banner, and the reason none of it belongs in the status bar.
//
// **A press that failed or was not sent is not a standing condition, so it is
// not here.** It pops up as a toast over everything — `raised.tsx`.
//
// # Why it is its own file
//
// `App.tsx` reached the 900 lines the gate refuses. This band is the one seam
// inside it that is already a seam: `App` decides what is true and this decides
// how a standing truth is drawn, so the props below are exactly the conditions
// and nothing about the surface underneath. The same remedy `protocol.ts` has
// taken three times.
//
// **Nothing here dismisses on a timer.** A person who has just saved a file, or
// gone to look at a worktree, is not looking at this window; a notice that
// expired while they were is one they cannot get back.

import type { ReactNode } from "react";
import { Alert, Button, ManifestNotice } from "@armada/components";
import type { ManifestReading, WorktreeReclaimed } from "@armada/protocol";
import type { Failure } from "@armada/shell";
import { FailureBlock } from "@armada/shell";
import { reclaimed, TakenNotice } from "@armada/screens";

export type StandingProps = {
  /** Fleet, where the one connection is not one. */
  fleet: Failure | null;
  /** What Fleet's last read of `armada.yml` came to. */
  manifestReading: ManifestReading | null;
  /** The reading already put away, by the instant Fleet read the file. */
  readingSeen: string | null;
  onReadingSeen: (at: string | null) => void;
  /** A clipboard write is silent, so the surface confirms it. */
  onCopied: (value: string) => void;
  /** Events Fleet dropped, and how many of them have been read. */
  missed: number;
  acknowledged: number;
  onAcknowledged: (missed: number) => void;
  /**
   * What a reclaim gave back. **An array**: the per-Job act always sets one
   * entry, and a bulk clear sets one per branch a base cannot reach — so a
   * sweep that kept three says so three times rather than once.
   */
  givenBack: WorktreeReclaimed[];
  onGivenBack: (given: WorktreeReclaimed[]) => void;
  /** A press a freeze took and holds, while it holds. */
  taken: { title: string; body: string; onDismiss: () => void } | null;
  /** A clone that finished after its dialog closed — `LocatedNotice`, held by Locate. */
  located?: ReactNode;
};

/** Everything true above the surface, in the order it is met. */
export function Standing({
  fleet,
  manifestReading,
  readingSeen,
  onReadingSeen,
  onCopied,
  missed,
  acknowledged,
  onAcknowledged,
  givenBack,
  onGivenBack,
  taken,
  located,
}: StandingProps) {
  return (
    <>
      {/* Fleet, when the one connection is not one. The status bar keeps the
          single line; this is the same reading with the four runtime-file
          answers and the log under it. */}
      {fleet === null ? null : <FailureBlock failure={fleet} onCopied={onCopied} />}

      {/* Fleet's own Manifest read. **Here and not in the status bar**, which
          the contract holds to three states and one colour and warns must not
          become a second alert surface. Put away by hand: whoever saved the
          file is looking at their editor. Nothing draws for a read with no
          news in it — `ManifestNotice` asks that itself. */}
      {manifestReading === null || readingSeen === manifestReading.at ? null : (
        <ManifestNotice
          reading={manifestReading}
          onDismiss={() => onReadingSeen(manifestReading.at)}
        />
      )}

      {missed <= acknowledged ? null : (
        <Alert
          tone="escalated"
          title="Events were dropped before Bridge saw them"
          action={
            <Button variant="ghost" size="sm" onClick={() => onAcknowledged(missed)}>
              Noted
            </Button>
          }
        >
          {`${missed} events will never arrive. Fleet resynced current state after each drop, so the list below is repaired.`}
        </Alert>
      )}

      {located}

      {/* What a reclaim gave back. **Neutral, because nothing is wrong** — a
          branch kept for holding work nothing has taken is the safe setting
          working, and drawing it in the escalation hue would tell somebody the
          act failed when it did exactly what it promised. Dismissed by hand: a
          directory and a branch are what a person goes and looks at, and a
          notice that vanished while they did is one they cannot get back.

          One line per entry rather than one Alert per entry: a bulk clear
          that kept several branches is one thing that happened, read at
          once, not a stack a person dismisses one at a time. */}
      {givenBack.length === 0 ? null : (
        <Alert
          tone="neutral"
          title={givenBack.length === 1 ? "Worktree reclaimed" : `${givenBack.length} worktrees reclaimed`}
          action={
            <Button variant="ghost" size="sm" onClick={() => onGivenBack([])}>
              Dismiss
            </Button>
          }
        >
          {givenBack.map((given) => (
            <p key={given.job_id}>{reclaimed(given)}</p>
          ))}
        </Alert>
      )}

      {taken === null ? null : <TakenNotice {...taken} />}
    </>
  );
}

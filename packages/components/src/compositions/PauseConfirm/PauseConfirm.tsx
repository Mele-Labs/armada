import { FilePenLine, Folder, Power, RotateCw } from "lucide-react";

import { Dialog } from "../../primitives/Dialog/Dialog";
import { Line, Mono } from "../PoolSlots/ConfirmLine";

export type PauseFacts = {
  /** The Job's branch, which the work is committed to. */
  branch: string;
  /** Uncommitted files, where the worktree holds any. Empty draws no commit line. */
  files: readonly string[];
  /** The slot it gives back. Absent where the read has not named it. */
  slot?: string;
  /** A Job that is running has a Drone and processes to stop; one at a gate has none. */
  running: boolean;
};

/**
 * What a pause does, as the git effects it has and nothing else. **A running
 * Job's Drone and its processes end, and its step restarts on Resume**; a Job
 * parked at a gate has no process, so only the commit and the release remain.
 * Shared by the dialog on the Board and in Job detail and by Cleanup's panel.
 */
export function PauseEffects({ branch, files, slot, running }: PauseFacts) {
  return (
    <>
      {running ? (
        <Line Glyph={Power} said="The Drone and every process under it end" word="Stops the Drone and its processes" />
      ) : null}
      {files.length > 0 ? (
        <Line
          Glyph={FilePenLine}
          said="git add --all, then git commit. Ignored files are left out and nothing is pushed"
          word={`Commits uncommitted files to branch ${branch} as a WIP commit`}
        >
          <Mono label="Uncommitted files" items={files} />
        </Line>
      ) : null}
      <Line Glyph={Folder} said="The pool takes the slot back for its next lease" word={`Releases ${slot ?? "its slot"}`} />
      {running ? (
        <Line Glyph={RotateCw} said="A Drone opens on the same step, and the restart costs one retry attempt" word="The step restarts on Resume" />
      ) : null}
    </>
  );
}

/** Pause, asked for before it is sent. Neutral: the work is saved and nothing is lost. */
export function PauseConfirm({
  facts,
  refused,
  onConfirm,
  onCancel,
}: {
  facts: PauseFacts;
  /** What Fleet refused, said where the press was. */
  refused?: string | undefined;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog open tone="neutral" title="Pause this job?" confirmLabel="Pause" onCancel={onCancel} onConfirm={onConfirm}>
      <PauseEffects {...facts} />
      {refused === undefined ? null : (
        <p className="armada-pause-refused" role="alert">
          {refused}
        </p>
      )}
    </Dialog>
  );
}

/**
 * Resume, the one confirm every act on a paused Job opens. **It resumes the
 * Job and sends nothing else**: the act that was pressed is pressed again after.
 * Where the pool is full the Job reads waiting for a slot, which its own state
 * says, so this says nothing about it.
 */
export function ResumeConfirm({
  branch,
  pressed,
  refused,
  onConfirm,
  onCancel,
}: {
  branch: string;
  /** Opened by an act on a paused Job, which is not sent. */
  pressed: boolean;
  refused?: string | undefined;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Dialog open tone="neutral" title="This job is paused" confirmLabel="Resume" onCancel={onCancel} onConfirm={onConfirm}>
      <Line Glyph={Folder} said="The branch goes back in a slot, as it was left" word={`Puts branch ${branch} back in a slot`} />
      {pressed ? (
        <Line Glyph={RotateCw} said="Press the act again once the job is resumed" word="Does not send the act you pressed" />
      ) : null}
      {refused === undefined ? null : (
        <p className="armada-pause-refused" role="alert">
          {refused}
        </p>
      )}
    </Dialog>
  );
}

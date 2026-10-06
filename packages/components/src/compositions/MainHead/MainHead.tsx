import { useState } from "react";
import { GitMerge, Hammer } from "lucide-react";

import { CHECK_OUTCOME } from "../../generated/vocabulary";
import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Main's state at the head of the merge line, as the hub draws it. **Drawn ahead of Fleet**: the
 * wire serves none of this yet, and the mock publishes it on a line.
 *
 * Green is an icon and its tooltip beside the heading. Red is a band under it that stays
 * where the panel is folded: the failing Check, the failing test and the merge that turned it
 * red, each a link, then either the Job that took it or the two ways to hand it to one.
 */

/** A Job, by its id and what it is called. */
export type HubJob = { id: string; title: string };

/** A Job the work can be sent back to. */
export type RecentJob = HubJob & { branch: string };

/** What turned main red. */
export type MainRed = {
  /**
   * What failed on main, as the forge reports it: a Manifest Check's name where the failing CI job
   * maps to one, otherwise the CI job's own name, and `unmapped` says which.
   */
  check: string;
  unmapped?: true;
  /** The failing test, where Armada could read one out of the log. Absent draws no row. */
  test?: string;
  /** Where the test is, at the merge. */
  testUrl?: string;
  merge: {
    number: number;
    url: string;
    branch: string;
    branchUrl: string;
    /** The Job whose pull request it was. Absent for a person's. */
    job?: HubJob;
  };
};

export type MainState =
  | { state: "green" }
  /** `taken` is the Job working on it: absent until one is. */
  | { state: "red"; red: MainRed; taken?: HubJob };

/** What the owner chose: a new Job with this brief, or an earlier Job to send the work back to. */
export type FixChoice = { kind: "new"; request: string } | { kind: "back"; job: string };

const MARK = 16;
const SMALL = 12;
const STROKE = 2;

/** Main's own state beside the heading. Green says nothing but the tooltip. */
export function MainMark({ main }: { main: MainState }) {
  const reading = CHECK_OUTCOME[main.state === "green" ? "passed" : "failed"];
  const Icon = reading?.icon ?? null;
  const said = main.state === "green" ? "Main is green" : "Main is red";
  if (Icon === null) return null;
  return (
    <Tooltip label={said} asChild>
      <span
        className="armada-main-mark"
        role="img"
        aria-label={said}
        style={reading?.statusToken ? { color: `var(${reading.statusToken})` } : undefined}
      >
        <Icon size={MARK} strokeWidth={STROKE} aria-hidden />
      </span>
    </Tooltip>
  );
}

/**
 * A Job that took main's red, or fixed it, as a mark beside its status badge. **An icon and its
 * tooltip, no phrase.** The hammer pulses while it works and holds still under reduced motion.
 */
export function FixingMainMark({ state, said }: { state: "fixing" | "fixed"; said: string }) {
  const Fixed = CHECK_OUTCOME.passed?.icon ?? null;
  return (
    <Tooltip label={said}>
      <span className="armada-fixing-mark" data-state={state} role="img" aria-label={said}>
        {state === "fixing" ? (
          <Hammer size={SMALL} strokeWidth={STROKE} aria-hidden />
        ) : Fixed === null ? null : (
          <Fixed size={SMALL} strokeWidth={STROKE} aria-hidden />
        )}
      </span>
    </Tooltip>
  );
}

export type MainRedBandProps = {
  main: Extract<MainState, { state: "red" }>;
  /** Earlier Jobs, newest first. The culprit's own is drawn first wherever it is among them. */
  recent: readonly RecentJob[];
  onOpenLink: (url: string) => void;
  /** Opens the failed Check's log. Absent, the Check is not a press. */
  onOpenCheck?: (check: string) => void;
  onOpenJob?: (jobId: string) => void;
  /** Told the owner's choice. The band draws the way back to a Job whether or not anything listens. */
  onFix?: (choice: FixChoice) => void;
};

export function MainRedBand({ main, recent, onOpenLink, onOpenCheck, onOpenJob, onFix }: MainRedBandProps) {
  const { red, taken } = main;
  const [asking, setAsking] = useState<"new" | "back" | null>(null);
  const link = (url: string) => (event: { preventDefault: () => void }) => {
    event.preventDefault();
    onOpenLink(url);
  };
  return (
    <div className="armada-main-red">
      <div className="armada-main-red__frame" role="status" aria-label="Main is red">
        <div className="armada-main-red__head">
          <span className="armada-main-red__title">Main is red</span>
          {taken !== undefined ? null : (
            <span className="armada-main-red__ask">
              <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("new")}>
                Dispatch a new Job
              </Button>
              <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("back")}>
                Send back to a Job
              </Button>
            </span>
          )}
        </div>
        <dl className="armada-main-red__rows mono">
          <dt>{labelOf(red)}</dt>
          <dd>
            {onOpenCheck === undefined ? (
              red.check
            ) : (
              <button type="button" className="armada-main-red__link mono" onClick={() => onOpenCheck(red.check)}>
                {red.check}
              </button>
            )}
          </dd>
          {red.test === undefined || red.testUrl === undefined ? null : (
            <>
              <dt>Test</dt>
              <dd>
                <a className="armada-main-red__link mono" href={red.testUrl} onClick={link(red.testUrl)}>
                  {red.test}
                </a>
              </dd>
            </>
          )}
          <dt>Broke in</dt>
          <dd className="armada-main-red__merge">
            <a className="armada-main-red__link mono" href={red.merge.url} onClick={link(red.merge.url)}>
              #{red.merge.number}
            </a>
            <a className="armada-main-red__link mono" href={red.merge.branchUrl} onClick={link(red.merge.branchUrl)}>
              {red.merge.branch}
            </a>
          </dd>
          {taken === undefined ? null : (
            <>
              <dt>Fixing</dt>
              <dd className="armada-main-red__taken">
                <Tooltip label="Working on it" asChild>
                  <span className="armada-fixing-mark" data-state="fixing" role="img" aria-label="Working on it">
                    <Hammer size={SMALL} strokeWidth={STROKE} aria-hidden />
                  </span>
                </Tooltip>
                {onOpenJob === undefined ? (
                  <span>{taken.title}</span>
                ) : (
                  <button type="button" className="armada-main-red__link" onClick={() => onOpenJob(taken.id)}>
                    {taken.title}
                  </button>
                )}
              </dd>
            </>
          )}
        </dl>
      </div>
      {asking === null ? null : (
        <FixDialog
          kind={asking}
          red={red}
          recent={recent}
          onCancel={() => setAsking(null)}
          onFix={(choice) => {
            setAsking(null);
            onFix?.(choice);
          }}
        />
      )}
    </div>
  );
}

/** What the first row is called: Armada adds Check meaning only where the CI job maps to a Manifest Check. */
const labelOf = (red: MainRed) => (red.unmapped === true ? "CI job" : "Check");

/** What goes with the work wherever it is sent: the Check, its test, its log and the merge. */
function Attached({ red }: { red: MainRed }) {
  return (
    <div className="armada-main-fix__attached" role="list" aria-label="Attached">
      <span role="listitem">
        <AttachmentChip from={labelOf(red)} filename={red.check} />
      </span>
      {red.test === undefined ? null : (
        <span role="listitem">
          <AttachmentChip from="Test" filename={red.test} />
        </span>
      )}
      <span role="listitem">
        <AttachmentChip from="Log" filename={`${red.check}.log`} />
      </span>
      <span role="listitem">
        <AttachmentChip from="Pull request" filename={`#${red.merge.number} ${red.merge.branch}`} />
      </span>
    </div>
  );
}

/** The brief a new Job starts from: bare facts, there to be edited. */
export function briefOf(red: MainRed): string {
  return [
    `${red.check} fails on main.`,
    ...(red.test === undefined ? [] : [`Test: ${red.test}`]),
    `Merged in #${red.merge.number} (${red.merge.branch}).`,
  ].join("\n");
}

/** The culprit's own Job first, then the rest as they came. */
export function culpritFirst(recent: readonly RecentJob[], culprit: HubJob | undefined): RecentJob[] {
  const own = recent.filter((one) => one.id === culprit?.id);
  return [...own, ...recent.filter((one) => one.id !== culprit?.id)];
}

function FixDialog({
  kind,
  red,
  recent,
  onCancel,
  onFix,
}: {
  kind: "new" | "back";
  red: MainRed;
  recent: readonly RecentJob[];
  onCancel: () => void;
  onFix: (choice: FixChoice) => void;
}) {
  const [request, setRequest] = useState(() => briefOf(red));
  const [job, setJob] = useState<string | null>(null);
  const jobs = culpritFirst(recent, red.merge.job);
  if (kind === "new") {
    return (
      <Dialog
        open
        tone="neutral"
        width="wide"
        title="Dispatch a Job to fix main"
        confirmLabel="Dispatch"
        confirmDisabled={request.trim() === ""}
        onCancel={onCancel}
        onConfirm={() => onFix({ kind: "new", request })}
      >
        <Attached red={red} />
        <Textarea label="Brief" rows={5} value={request} onChange={(event) => setRequest(event.target.value)} />
      </Dialog>
    );
  }
  return (
    <Dialog
      open
      tone="neutral"
      width="wide"
      title="Send the work back to a Job"
      confirmLabel="Send back"
      confirmDisabled={job === null}
      onCancel={onCancel}
      onConfirm={() => job !== null && onFix({ kind: "back", job })}
    >
      <Attached red={red} />
      <RadioGroup label="Job">
        {jobs.map((one) => (
          <Radio key={one.id} name="main-fix-job" value={one.id} checked={job === one.id} onChange={() => setJob(one.id)}>
            <span className="armada-main-fix__job">
              <span>{one.title}</span>
              <span className="mono armada-main-fix__branch">{one.branch}</span>
              {one.id === red.merge.job?.id ? (
                <Tooltip label={`Merged #${red.merge.number}, the pull request that turned main red`} asChild>
                  <span className="armada-main-fix__culprit" role="img" aria-label={`Merged #${red.merge.number}`}>
                    <GitMerge size={SMALL} strokeWidth={STROKE} aria-hidden />
                  </span>
                </Tooltip>
              ) : null}
            </span>
          </Radio>
        ))}
      </RadioGroup>
    </Dialog>
  );
}

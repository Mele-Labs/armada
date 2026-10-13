import { Fragment, useState } from "react";
import { Box, GitMerge, Hammer, SquareTerminal } from "lucide-react";

import { CHECK_OUTCOME } from "../../generated/vocabulary";
import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Dialog } from "../../primitives/Dialog/Dialog";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Main's state at the head of the merge line, as the hub draws it. Fleet serves it as the line's
 * `hub` since protocol 23.41, and `packages/screens/src/merge-line.ts` folds it onto these types.
 *
 * Green is an icon and its tooltip beside the heading. Red is a band under it that stays
 * where the panel is folded: the failing Check, the failing test and the merge that turned it
 * red, each a link, then either the Job that took it or the two ways to hand it to one.
 */

/** A Job, by its id and what it is called. */
export type HubJob = { id: string; title: string };

/** A Job the work can be sent back to. */
export type RecentJob = HubJob & { branch: string };

/** A Session, by its id and what it is called. */
export type HubSession = { id: string; title: string };

/** A Session the work can be sent back to, as a message. */
export type RecentSession = HubSession & { branch?: string };

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
  /** Where the test is, at the merge. Absent draws the test as text. */
  testUrl?: string;
  /** Any other CI job that failed, each as the first is. Absent is the one. */
  also?: readonly { check: string; unmapped?: true; test?: string }[];
  /** When main first read red, RFC 3339, kept while it stays red. Absent from a Fleet before it carried it. */
  since?: string;
  /** The pull request that merged. Absent where the forge named none, as a direct push does not. */
  merge?: {
    number: number;
    /** Absent where only the merge commit's message named the pull request: its number is text. */
    url?: string;
    /** Absent where the forge did not say. */
    branch?: string;
    /** Absent draws the branch as text. */
    branchUrl?: string;
    /** The Job whose pull request it was. Absent for a person's. */
    job?: HubJob;
    /** The Session whose pull request it was. Absent for a Job's or a person's. Wins over `job` where both are named. */
    session?: HubSession;
  };
};

/** A newer commit on main whose CI is still running: its pull request where the forge named one, else the commit. */
export type MainChecking = { commit: string; number?: number; url?: string };

export type MainState =
  /** `checking` non-empty is only checks running on main: the mark says so, and no band is drawn. */
  | { state: "green"; checking?: readonly MainChecking[] }
  /**
   * `taken` is the Job working on it: absent until one is. **`checking` non-empty is a held red**: a
   * newer run may already have fixed it, so the band is caution, says so, and offers no way to hand
   * it to a Job. Absent or empty is a red to act on.
   */
  | { state: "red"; red: MainRed; taken?: HubJob; checking?: readonly MainChecking[] };

/**
 * What the owner chose: a new Job with this brief, an earlier Job to send the work back to, or a
 * Session to send it to as a message.
 */
export type FixChoice =
  | { kind: "new"; request: string }
  | { kind: "back"; job: string }
  | { kind: "session"; session: string; message: string };

const MARK = 16;
const SMALL = 12;
const STROKE = 2;

/** Main's own state beside the heading. Green says nothing but the tooltip. */
export function MainMark({ main }: { main: MainState }) {
  const reading = CHECK_OUTCOME[main.state === "green" ? "passed" : "failed"];
  const Icon = reading?.icon ?? null;
  const checking = (main.checking?.length ?? 0) > 0;
  const held = main.state === "red" && checking;
  const said =
    main.state === "green"
      ? checking
        ? "Main is green, new checks are running"
        : "Main is green"
      : held
        ? "Main is red, new checks are running"
        : "Main is red";
  if (Icon === null) return null;
  return (
    <Tooltip label={said} asChild>
      <span
        className="armada-main-mark"
        data-checking={(main.state === "green" && checking) || undefined}
        role="img"
        aria-label={said}
        style={held ? { color: "var(--notice-caution)" } : reading?.statusToken ? { color: `var(${reading.statusToken})` } : undefined}
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
  /** Earlier Sessions, newest first. The culprit's own is drawn first wherever it is among them. Absent is none. */
  recentSessions?: readonly RecentSession[];
  onOpenLink: (url: string) => void;
  /** Opens the failed Check's log. Absent, the Check is not a press. */
  onOpenCheck?: (check: string) => void;
  onOpenJob?: (jobId: string) => void;
  /** Opens the Session that merged. Absent, its title is not a press. */
  onOpenSession?: (sessionId: string) => void;
  /**
   * Told the owner's choice. **The two ways to hand the red to a Job draw only where something
   * listens**: a Fleet that cannot take one offers none, and a button that does nothing is worse.
   */
  onFix?: (choice: FixChoice) => void;
};

export function MainRedBand({ main, recent, recentSessions = [], onOpenLink, onOpenCheck, onOpenJob, onOpenSession, onFix }: MainRedBandProps) {
  const { red, taken } = main;
  const checking = main.checking ?? [];
  const held = checking.length > 0;
  // The acts draw only where something listens, and not where a held red or a Job on it makes them moot.
  const asks = !held && taken === undefined && onFix !== undefined;
  const [asking, setAsking] = useState<"new" | "back" | null>(null);
  const link = (url: string) => (event: { preventDefault: () => void }) => {
    event.preventDefault();
    onOpenLink(url);
  };
  return (
    <div className="armada-main-red" data-held={held || undefined}>
      <div className="armada-main-red__frame" role="status" aria-label={held ? "New checks are running on main" : "Main is red"}>
        <div className="armada-main-red__head">
          {held ? (
            <span className="armada-main-red__title">
              New checks are running on main:{" "}
              {checking.map((one, at) => (
                <Fragment key={one.commit}>
                  {at === 0 ? null : " "}
                  {one.number === undefined ? (
                    <span className="mono">{one.commit.slice(0, 10)}</span>
                  ) : one.url === undefined ? (
                    <span>#{one.number}</span>
                  ) : (
                    <a className="armada-main-red__link mono" href={one.url} onClick={link(one.url)}>
                      #{one.number}
                    </a>
                  )}
                </Fragment>
              ))}
            </span>
          ) : (
            <span className="armada-main-red__title">Main is red</span>
          )}
          {!asks ? null : (
            <span className="armada-main-red__ask">
              <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("new")}>
                Dispatch a new Job
              </Button>
              {red.merge !== undefined ? null : (
                <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("back")}>
                  Send back to…
                </Button>
              )}
            </span>
          )}
        </div>
        <dl className="armada-main-red__rows mono">
          {[{ check: red.check, unmapped: red.unmapped, test: red.test, testUrl: red.testUrl }, ...(red.also ?? [])].map(
            (job, at) => (
              <Fragment key={`${job.check}:${at}`}>
                <dt>{job.unmapped === true ? "CI job" : "Check"}</dt>
                <dd>
                  {onOpenCheck === undefined ? (
                    job.check
                  ) : (
                    <button type="button" className="armada-main-red__link mono" onClick={() => onOpenCheck(job.check)}>
                      {job.check}
                    </button>
                  )}
                </dd>
                {job.test === undefined ? null : (
                  <>
                    <dt>Test</dt>
                    <dd>
                      {"testUrl" in job && job.testUrl !== undefined ? (
                        <a className="armada-main-red__link mono" href={job.testUrl} onClick={link(job.testUrl)}>
                          {job.test}
                        </a>
                      ) : (
                        job.test
                      )}
                    </dd>
                  </>
                )}
              </Fragment>
            ),
          )}
          {red.merge === undefined ? null : (
            <>
              <dt>Broke in</dt>
              <dd className="armada-main-red__merge">
                {red.merge.url === undefined ? (
                  <span>#{red.merge.number}</span>
                ) : (
                  <a className="armada-main-red__link mono" href={red.merge.url} onClick={link(red.merge.url)}>
                    #{red.merge.number}
                  </a>
                )}
                {red.merge.branch === undefined ? null : red.merge.branchUrl === undefined ? (
                  <span>{red.merge.branch}</span>
                ) : (
                  <a className="armada-main-red__link mono" href={red.merge.branchUrl} onClick={link(red.merge.branchUrl)}>
                    {red.merge.branch}
                  </a>
                )}
                <MergeOwner merge={red.merge} {...(onOpenJob === undefined ? {} : { onOpenJob })} {...(onOpenSession === undefined ? {} : { onOpenSession })} />
                {!asks ? null : (
                  <Button variant="secondary" size="sm" ground="sunken" onClick={() => setAsking("back")}>
                    {red.merge.session !== undefined || red.merge.job !== undefined ? "Send back" : "Send back to…"}
                  </Button>
                )}
              </dd>
            </>
          )}
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
          recentSessions={recentSessions}
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

/**
 * Who merged: the Session that owns the pull request, else the Job. Its registry icon says which,
 * as a pull request row on the line draws it, and its title opens it. Nobody known draws nothing.
 */
function MergeOwner({
  merge,
  onOpenJob,
  onOpenSession,
}: {
  merge: NonNullable<MainRed["merge"]>;
  onOpenJob?: (jobId: string) => void;
  onOpenSession?: (sessionId: string) => void;
}) {
  const owner = merge.session !== undefined ? { kind: "Session", Icon: SquareTerminal, one: merge.session, open: onOpenSession } : merge.job !== undefined ? { kind: "Job", Icon: Box, one: merge.job, open: onOpenJob } : undefined;
  if (owner === undefined) return null;
  const { kind, Icon, one, open } = owner;
  return (
    <span className="armada-main-red__owner">
      <Tooltip label={kind} asChild>
        <span className="armada-main-red__owner-mark" role="img" aria-label={kind}>
          <Icon size={SMALL} strokeWidth={STROKE} aria-hidden />
        </span>
      </Tooltip>
      {open === undefined ? (
        <span>{one.title}</span>
      ) : (
        <button type="button" className="armada-main-red__link" onClick={() => open(one.id)}>
          {one.title}
        </button>
      )}
    </span>
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
      {red.merge === undefined ? null : (
        <span role="listitem">
          <AttachmentChip from="Pull request" filename={[`#${red.merge.number}`, red.merge.branch].filter(Boolean).join(" ")} />
        </span>
      )}
    </div>
  );
}

/** The brief a new Job starts from: bare facts, there to be edited. */
export function briefOf(red: MainRed): string {
  return [
    `${red.check} fails on main.`,
    ...(red.test === undefined ? [] : [`Test: ${red.test}`]),
    ...(red.merge === undefined ? [] : [`Merged in #${red.merge.number}${red.merge.branch === undefined ? "" : ` (${red.merge.branch})`}.`]),
  ].join("\n");
}

/** The culprit's own Job or Session first, then the rest as they came. */
export function culpritFirst<T extends { id: string }>(recent: readonly T[], culprit: { id: string } | undefined): T[] {
  const own = recent.filter((one) => one.id === culprit?.id);
  return [...own, ...recent.filter((one) => one.id !== culprit?.id)];
}

/** The one picked in the send-back dialog: a Session or a Job. */
type Picked = { kind: "session" | "job"; id: string } | null;

function FixDialog({
  kind,
  red,
  recent,
  recentSessions,
  onCancel,
  onFix,
}: {
  kind: "new" | "back";
  red: MainRed;
  recent: readonly RecentJob[];
  recentSessions: readonly RecentSession[];
  onCancel: () => void;
  onFix: (choice: FixChoice) => void;
}) {
  const [request, setRequest] = useState(() => briefOf(red));
  const jobs = culpritFirst(recent, red.merge?.job);
  const sessions = culpritFirst(recentSessions, red.merge?.session);
  // The culprit is chosen before the owner looks: the Session if one merged it, else the Job.
  const [picked, setPicked] = useState<Picked>(() =>
    red.merge?.session !== undefined && sessions[0]?.id === red.merge.session.id
      ? { kind: "session", id: red.merge.session.id }
      : red.merge?.job !== undefined && jobs[0]?.id === red.merge.job.id
        ? { kind: "job", id: red.merge.job.id }
        : null,
  );
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
  const mark = (id: string, owner: { id: string } | undefined) =>
    red.merge === undefined || owner === undefined || id !== owner.id ? null : (
      <Tooltip label={`Merged #${red.merge.number}, the pull request that turned main red`} asChild>
        <span className="armada-main-fix__culprit" role="img" aria-label={`Merged #${red.merge.number}`}>
          <GitMerge size={SMALL} strokeWidth={STROKE} aria-hidden />
        </span>
      </Tooltip>
    );
  return (
    <Dialog
      open
      tone="neutral"
      width="wide"
      title="Send the work back"
      confirmLabel="Send back"
      confirmDisabled={picked === null || (picked.kind === "session" && request.trim() === "")}
      onCancel={onCancel}
      onConfirm={() => {
        if (picked === null) return;
        onFix(picked.kind === "session" ? { kind: "session", session: picked.id, message: request } : { kind: "back", job: picked.id });
      }}
    >
      <Attached red={red} />
      {sessions.length === 0 ? null : (
        <RadioGroup label="Sessions">
          {sessions.map((one) => (
            <Radio key={one.id} name="main-fix-session" value={one.id} checked={picked?.kind === "session" && picked.id === one.id} onChange={() => setPicked({ kind: "session", id: one.id })}>
              <span className="armada-main-fix__job">
                <span>{one.title}</span>
                {one.branch === undefined ? null : <span className="mono armada-main-fix__branch">{one.branch}</span>}
                {mark(one.id, red.merge?.session)}
              </span>
            </Radio>
          ))}
        </RadioGroup>
      )}
      {jobs.length === 0 ? null : (
        <RadioGroup label="Jobs">
          {jobs.map((one) => (
            <Radio key={one.id} name="main-fix-job" value={one.id} checked={picked?.kind === "job" && picked.id === one.id} onChange={() => setPicked({ kind: "job", id: one.id })}>
              <span className="armada-main-fix__job">
                <span>{one.title}</span>
                <span className="mono armada-main-fix__branch">{one.branch}</span>
                {mark(one.id, red.merge?.job)}
              </span>
            </Radio>
          ))}
        </RadioGroup>
      )}
      {picked?.kind === "session" ? <Textarea label="Message" rows={5} value={request} onChange={(event) => setRequest(event.target.value)} /> : null}
    </Dialog>
  );
}

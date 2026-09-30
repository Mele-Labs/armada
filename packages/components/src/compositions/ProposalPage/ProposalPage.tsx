import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useEffect } from "react";

import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Card, CardContent, CardFooter, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { ErrorNotice } from "../../errors/ErrorNotice/ErrorNotice";
import type { DebugPayload } from "../../errors/ErrorNotice/ErrorNotice";
import { ACTION } from "../../actions";
import { JOB_STATUS } from "../../generated/vocabulary";
import { Waiting } from "./Waiting";
import type { ProposalWatch } from "./Waiting";

/** The wait is its own file; the type it reads goes with it. */
export type { ProposalWatch } from "./Waiting";

/**
 * One proposal, from the press that sent it to what it became.
 *
 * **A destination and not a state of the form**, and why it is not the Job page
 * either, is the owner's decision of 30 Sep 2026, *the wait is a destination*.
 *
 * Nothing here invents a Job: no id, no record and no Job-shaped reading exists
 * until Fleet answers with one.
 *
 * **It does not fill the proposal in progressively and must not look as though
 * it does.** The Jobs arrive whole, once, at the end; what moves here is the
 * call's own progress, which is a fact about the wait rather than a preview of
 * the answer.
 */
export type ProposalPageProps = {
  /**
   * The request, as it was sent — never summarised and never trimmed, because
   * the proposal is only readable against what was asked.
   *
   * Absent where this window did not send it: Bridge and Fleet have independent
   * lifetimes, so a window reopened mid-call knows one is out and not what it
   * was about. That draws no quote rather than a sentence standing in for one.
   */
  request?: string;
  /** Where the one call has got to. */
  proposal: ProposalAnswer;
  /**
   * Stop the call. **Kills it rather than leaving the screen** — a wait
   * abandoned leaves the proposer running inside Fleet and spending, with
   * nobody left to read what it decides.
   *
   * Absent draws no control rather than a dead one.
   */
  onStop?: () => void;
  /**
   * How long a wait may run before the screen says it is taking longer than
   * expected, in milliseconds.
   *
   * **A prompt, not a limit.** Nothing happens at the mark: the call runs until
   * Fleet's own budget or until somebody presses stop. It is the caller's
   * because what counts as long is a property of the deployment.
   */
  slowAfterMs?: number;
  /**
   * Open one of the Jobs the request became. **Called by the screen itself
   * where the proposal is one Job**, which is how it becomes the Job page.
   */
  onOpen: (jobId: string) => void;
  /**
   * Release the Job at the head of the proposal, which is what starts the work.
   *
   * **Offered on one row and never on two.** Only the head is at its gate — the
   * rest of a chain reach theirs as the one before them completes.
   */
  onApprove: (jobId: string) => void;
  /**
   * Jobs whose approval is out, by id. The control says `Approving` and goes
   * dead: approving twice does not spawn twice, but a control that looks
   * unpressed invites the second press and says nothing about the first.
   */
  approving?: readonly string[];
  /**
   * Back to the composer, holding the request. **The way on from all three
   * endings that produced no Job**, because what a person does about each of
   * them is the same act on the same words.
   */
  onEdit: () => void;
  /** Back to the composer, empty — a fresh request after a proposal landed. */
  onAnother: () => void;
  /**
   * The way off the screen, drawn at the head's trailing edge. The caller's
   * control; absent draws none.
   */
  close?: ReactNode;
  /** What the surface is told after a clipboard write, so it can raise a toast. */
  onCopied?: (what: string) => void;
};

/**
 * Where the one call has got to.
 *
 * **Six states and no seventh.** There is no partial proposal: the call is
 * asked once and answers once, and `reading`'s `watch` describes the call rather
 * than the answer.
 */
export type Proposal =
  /** Nothing asked. Where a reset returns to, and no screen draws it. */
  | { at: "unasked" }
  /**
   * Asked, and waiting. Absent `watch` is every moment before Fleet's first
   * message and every Fleet too old to send one — the wait is drawn without it
   * rather than not drawn.
   */
  | { at: "reading"; watch?: ProposalWatch }
  /** Answered. Every Job here exists already, at `awaiting_approval`. */
  | { at: "proposed"; jobs: readonly ProposedJob[] }
  /** No workflow resolved. The request is unchanged and no Job was created. */
  | { at: "unresolved" }
  /**
   * Somebody stopped it. **Not a failure and not a refusal**: they pressed a
   * control Armada offered them, nothing was created, and what they typed comes
   * back.
   */
  | { at: "stopped" }
  /** The call could not be made. A fault, and it carries a code. */
  | { at: "faulted"; code: string; message: ReactNode; payload?: DebugPayload };

/** A proposal this screen can be drawn on. `unasked` is no screen at all. */
export type ProposalAnswer = Exclude<Proposal, { at: "unasked" }>;

/** One Job the request became. **No scope, because none was proposed.** */
export type ProposedJob = {
  id: string;
  /** What the proposer called it. Nobody typed this. */
  title: string;
  /**
   * The workflow's name, resolved by the caller. Never the id: an id in a
   * proposal is the one field a person cannot check.
   */
  workflow: string;
  /**
   * The Job's own status off the wire, `awaiting_approval` at this gate.
   * **Carried rather than assumed** — the Job exists before this screen draws
   * it, so the badge says what Fleet says.
   */
  status: string;
};

/**
 * What the screen is called, one heading per ending.
 *
 * **The heading is the state**, so the first line answers what happened rather
 * than making somebody read the body for it.
 */
const HEAD: Record<ProposalAnswer["at"], string> = {
  reading: "Reading the request",
  proposed: "What the request became",
  unresolved: "No workflow fits this request",
  stopped: "You stopped the proposer",
  faulted: "The request was not read",
};

/**
 * What a row's controls are called. `actions.toml` is the authority on the verb
 * and the binding, and `keys.ts` in `@armada/screens` reads the same rows for
 * the Board's own `awaiting_approval` row — one act, one word.
 */
const REVIEW = ACTION["review"];
const APPROVE = ACTION["approve"];

/** The status a Job is at when its gate is somebody's to release. */
const AT_THE_GATE = "awaiting_approval";

/** Said on every ending that produced nothing: the fact a person most needs. */
const NOTHING_CREATED = "Nothing was created and the request is unchanged.";

export function ProposalPage({
  request,
  proposal,
  onStop,
  slowAfterMs,
  onOpen,
  onApprove,
  approving = [],
  onEdit,
  onAnother,
  close,
  onCopied,
}: ProposalPageProps) {
  const answered = proposal.at === "proposed";

  return (
    <Card className="armada-proposal-page">
      <CardHeader>
        <CardTitle>{HEAD[proposal.at]}</CardTitle>
        {close}
      </CardHeader>
      <CardContent>
        <div className="armada-proposal-page__body">
          {/* What was asked, kept on screen for the whole of the proposal's
              life: the wait and the answer are both read against it. */}
          {request === undefined ? null : (
            <p className="armada-proposal-page__asked">{request}</p>
          )}

          {proposal.at === "reading" ? (
            <Waiting
              {...(proposal.watch === undefined ? {} : { watch: proposal.watch })}
              {...(onStop === undefined ? {} : { onStop })}
              {...(slowAfterMs === undefined ? {} : { slowAfterMs })}
            />
          ) : null}

          {proposal.at === "proposed" ? (
            <Answered
              jobs={proposal.jobs}
              onOpen={onOpen}
              onApprove={onApprove}
              approving={approving}
            />
          ) : null}

          {/* Armada working rather than failing: Fleet read the request and
              declined, so no red and no code. The way on is the request. */}
          {proposal.at === "unresolved" ? (
            <div className="armada-proposal-page__declined" role="status">
              <p className="armada-proposal-page__declined-head">{NOTHING_CREATED}</p>
              {/* The next move only. What a workflow is was true before the
                  refusal, so it belongs to the composer's own guide. */}
              <p className="armada-proposal-page__declined-body">
                Edit the request and dispatch again, or name the workflow yourself under
                Settings.
              </p>
            </div>
          ) : null}

          {/* Nobody's failure, so the decline's treatment and not the fault's.
              `refusing.rs` gives the code its own name for exactly this: red
              here would say Armada broke, when somebody pressed a control. */}
          {proposal.at === "stopped" ? (
            <div className="armada-proposal-page__declined" role="status">
              <p className="armada-proposal-page__declined-head">{NOTHING_CREATED}</p>
              <p className="armada-proposal-page__declined-body">
                The call was killed rather than abandoned, so nothing is still running and
                nothing more is being spent.
              </p>
            </div>
          ) : null}

          {/* Armada failing, so the error treatment and its code. Inline,
              because a proposer that could not be called stops this screen and
              reaches nothing else. */}
          {proposal.at === "faulted" ? (
            <ErrorNotice
              kind="fault"
              placement="inline"
              code={proposal.code}
              message={
                <>
                  {proposal.message} {NOTHING_CREATED}
                </>
              }
              {...(proposal.payload === undefined ? {} : { payload: proposal.payload })}
              onCopied={onCopied}
              act={
                /* The act named, not repeated: the footer performs it, and a
                   second button here would be two controls for one act. The
                   sentence says the part the button cannot — the fault said
                   nothing about the request. */
                <span className="armada-proposal-page__act">
                  Dispatch it again. The request is not what failed.
                </span>
              }
            />
          ) : null}
        </div>
      </CardContent>

      {/* Nothing in the footer while the call is out: the one act on a proposal
          in flight is the stop, and it is on the wait it belongs to. */}
      {proposal.at === "reading" ? null : (
        <CardFooter className="armada-proposal-page__foot">
          {answered ? (
            <Button variant="secondary" onClick={onAnother}>
              Dispatch another
            </Button>
          ) : (
            /* One control for all three endings that produced nothing: the act
               is the same on the same words, and the words are on the composer
               this goes back to. */
            <Button variant="primary" onClick={onEdit}>
              Edit the request
            </Button>
          )}
        </CardFooter>
      )}
    </Card>
  );
}

/**
 * What the request became.
 *
 * **One Job is opened, not listed.** A list of one is not a list, and the
 * screen becomes the Job the proposer drafted — so this asks the caller to open
 * it and draws the row meanwhile. The caller navigates; a component that routed
 * would be a second place navigation happens.
 *
 * **The order is the whole of the graph.** A proposal of several is a chain,
 * each member waiting on the one before it reaching `completed_success`, so
 * position carries it and no second field restates it.
 */
function Answered({
  jobs,
  onOpen,
  onApprove,
  approving,
}: {
  jobs: readonly ProposedJob[];
  onOpen: (jobId: string) => void;
  onApprove: (jobId: string) => void;
  approving: readonly string[];
}) {
  const several = jobs.length > 1;
  const alone = several ? undefined : jobs[0];
  // **Keyed on the Job's id alone.** The caller rebuilds `onOpen` every render,
  // so listing it would reopen the Job on every tick of the window's clock.
  useEffect(() => {
    if (alone !== undefined) onOpen(alone.id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [alone?.id]);

  return (
    <div className="armada-proposal-page__answered">
      <p className="armada-proposal-page__became">
        {several
          ? "The request became a chain of jobs, in this order."
          : "The request became one job, and it is open."}
      </p>

      <ol className="armada-proposal-page__jobs">
        {jobs.map((job, index) => (
          <li className="armada-proposal-page__job" key={job.id}>
            {several ? (
              <span className="armada-proposal-page__ordinal mono" aria-hidden="true">
                {index + 1}
              </span>
            ) : null}
            <div className="armada-proposal-page__job-body">
              <span className="armada-proposal-page__title">{job.title}</span>
              <span className="armada-proposal-page__workflow">{job.workflow}</span>
              {several && index > 0 ? (
                <span className="armada-proposal-page__waits">{`Waits on job ${index}.`}</span>
              ) : null}
            </div>
            <AtTheGate status={job.status} />

            <div className="armada-proposal-page__job-acts">
              {/* Opens the Job, where the title is not enough and the Job
                  itself is what somebody wants to read. */}
              <Button
                variant="secondary"
                size="sm"
                onClick={() => onOpen(job.id)}
                aria-label={`${REVIEW === undefined ? "Review" : REVIEW.verb} ${job.title}`}
              >
                {REVIEW === undefined ? "Review" : REVIEW.verb}
              </Button>

              {/* The gate, on the one row that holds it. The status is Fleet's
                  and is checked rather than assumed: a Job already released
                  draws no second one. */}
              {index === 0 && job.status === AT_THE_GATE ? (
                <Releasing job={job} out={approving.includes(job.id)} onApprove={onApprove} />
              ) : null}
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}

/**
 * The gate, as a control. **The label and the accessible name are the same
 * words** — a button reading `Approving` under a name saying `Approve` tells a
 * screen reader the press is still there to make.
 */
function Releasing({
  job,
  out,
  onApprove,
}: {
  job: ProposedJob;
  /** This Job's approval is in flight. */
  out: boolean;
  onApprove: (jobId: string) => void;
}) {
  const verb = APPROVE === undefined ? "Approve" : APPROVE.verb;
  const says = out ? "Approving" : verb;
  return (
    <Button
      variant="primary"
      size="sm"
      pending={out}
      onClick={() => onApprove(job.id)}
      aria-label={`${says} ${job.title}`}
    >
      {says}
    </Button>
  );
}

/**
 * The badge on a proposed Job. **The Job's own status, not this screen's idea
 * of it** — every one of these exists on the board already, so a word hardcoded
 * here would be Bridge asserting something it was told.
 */
function AtTheGate({ status }: { status: string }) {
  const badge = badgeOf(status);
  if (badge === null) return null;
  return (
    <Badge status={badge.status} icon={badge.icon}>
      {badge.verb}
    </Badge>
  );
}

/**
 * A status as a badge draws it, from the generated vocabulary rather than typed
 * here — a second copy of a status word is a second vocabulary. `null` where
 * the registry carries no verb, glyph or token, which draws no badge rather
 * than an invented one.
 */
function badgeOf(status: string): { status: string; icon: LucideIcon; verb: string } | null {
  const rendering = JOB_STATUS[status];
  if (rendering === undefined) return null;
  const { badgeStatus, icon, verb } = rendering;
  if (badgeStatus === null || icon === null || verb === null) return null;
  return { status: badgeStatus, icon, verb };
}

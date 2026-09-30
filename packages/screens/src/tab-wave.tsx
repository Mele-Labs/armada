// The wave a Job dispatched — the passes of it, who needs you, the graph and
// the list. `#1544`.
//
// **Graph by default, List available** — Plan's own two words and its one
// remembered choice (owner, 30 Sep 2026), since on an Epic Job the wave is
// what Plan draws. The list is `ActiveJobsList` and `JobRowStacked`, the row
// every other surface draws a Job as.
//
// **Who needs you is a list of lines, and the answer is in the Job's panel**
// (owner, 30 Sep 2026: "both"). A line opens the Job; `JobAnswer` below is
// what the panel puts at its top — the existing Judge card and the dock's own
// command card, never a second shape for either.

import {
  ActiveJobsList,
  DockQuestions,
  FactChip,
  JobRowStacked,
  JudgeQuestion,
  Tabs,
  Tooltip,
  WaveCanvas,
} from "@armada/components";
import { JOB_STATUS } from "@armada/components/src/generated/vocabulary";
import { useCallback, useMemo, useState } from "react";

import type {
  CommandAnswer,
  JobDetail as JobWhole,
  JobSummary,
  JudgeAnswer,
  RepositorySummary,
  StepDetail,
} from "@armada/protocol";

import { answerNamed } from "./copy";
import { dockQuestionsOf } from "./dock-questions";
import type { JobDraft } from "./draft/held";
import type { WaveJobView, WaveView } from "./draft/wave";
import type { Outstanding } from "./outstanding";
import { PLAN_VIEWS, PLAN_VIEW_LABEL, type PlanView } from "./plan-view";
import { Eyebrow } from "./regions";
import { waveRunOf, waveSpentSaid, waveStandingOf, waveTasksSaid } from "./wave";

/** Why nothing can be answered from a window that is not live. */
const NOT_LIVE = "This window is not live, so nothing can be sent.";

type JudgeAsking = Extract<Outstanding, { kind: "judge" }>;

export type WaveRegionProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** What this moment's boards draw that Fleet cannot serve yet. */
  draft?: JobDraft;
  /** The Board's own rows, which is where a wave with no draft is derived from. */
  board: readonly JobSummary[];
  /** Every question waiting on a person, as main gathers them. */
  questions: readonly Outstanding[];
  repositories: readonly RepositorySummary[];
  now: number;
  /** Nothing is live, so every answer is refused. */
  stale: boolean;
  /** An act on one of these Jobs is already out. */
  acting: boolean;
  /** Graph or List — Plan's own choice, remembered per viewer by the caller. */
  view: PlanView;
  onView: (view: PlanView) => void;
  onOpenJob: (jobId: string) => void;
  onAnswerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  onAnswerCommand: (
    jobId: string,
    call: string,
    answer: CommandAnswer,
    note?: string,
    rule?: string,
  ) => void;
  /** The step that recorded the split, where one did. Its Judge is read off it. */
  planStep?: StepDetail;
};

/**
 * The wave to draw — **the moment's own, and nothing derived**.
 *
 * `dispatched_by` is all the wire has, and it is the same field a landing
 * order is read from: a parent whose three Jobs land in turn and a parent
 * whose five wait on each other are indistinguishable through it. Deriving a
 * wave from it drew both regions over the members moments, where the landing
 * band is the right one. So Bridge draws a wave only where something says this
 * Job is one, which today is the mock and after `#1545` is the wire.
 *
 * `waveOf` is the derivation that promotion turns on, kept and tested beside
 * its type. It is not reached from here until the wire can tell the two apart.
 */
export function waveReadingOf(
  _whole: JobWhole | null,
  draft: JobDraft | undefined,
  _board: readonly JobSummary[],
): WaveView | undefined {
  return draft?.wave;
}

/**
 * Which wave the Job is on, and how many it may run.
 *
 * Read off the wire: `StepDetail.pass` is which pass of how many, on the step
 * whose verdict routes back. A workflow that does not loop says nothing rather
 * than drawing a wave of one.
 */
export function loopOf(whole: JobWhole | null): { number: number; of: number } | undefined {
  if (whole === null) return undefined;
  const closes = whole.steps.find(
    (step) => step.pass !== undefined && step.verdict_routing_target !== undefined,
  );
  return closes?.pass;
}

/**
 * The loop in words — `Wave 2 of up to 5`, or only `Up to 5 waves` beside a
 * strip that already names the wave.
 */
export function loopSaid(whole: JobWhole | null, stripped = false): string | undefined {
  const pass = loopOf(whole);
  if (pass === undefined) return undefined;
  return stripped
    ? `Up to ${String(pass.of)} waves`
    : `Wave ${String(pass.number)} of up to ${String(pass.of)}`;
}

/** What each Job waits on, off the wave's own ids. */
export function waitsOf(wave: WaveView, job: WaveJobView): WaveJobView[] {
  return job.waits_on.flatMap((id) => {
    const held = wave.jobs.find((one) => one.job === id);
    return held === undefined ? [] : [held];
  });
}

/**
 * What waits on this Job. **Worked out from the others' `waits_on`**, which is
 * the one direction the plan records.
 */
export function blocksOf(wave: WaveView, job: WaveJobView): WaveJobView[] {
  return wave.jobs.filter((one) => one.waits_on.includes(job.job));
}

/**
 * What a Job that needs you is waiting on you for, in the words a line says.
 * `undefined` for a Job that needs nobody.
 */
function waitingOnSaid(
  job: WaveJobView,
  mine: readonly Outstanding[],
  needs: ReadonlySet<string>,
): string | undefined {
  const asked = mine.filter((one) => one.kind !== "helm" && one.job_id === job.job);
  if (asked.some((one) => one.kind === "judge")) return "your answer to the Judge";
  if (asked.some((one) => one.kind === "command")) return "you to allow a command";
  if (!needs.has(job.job)) return undefined;
  return job.status === "awaiting_review" ? "your review" : (JOB_STATUS[job.status]?.verb ?? job.status);
}

/** One Job of the wave as a row: the list's, the Needs you line's, and the panel's. */
export function WaveRow({
  job,
  fields,
  bare = false,
  onOpen,
}: {
  job: WaveJobView;
  fields: { label: string; value: string; quiet?: boolean }[];
  /** Leave the handle off: a line that only has to say which Job and why. */
  bare?: boolean;
  onOpen: () => void;
}) {
  const rendering = JOB_STATUS[job.status];
  if (rendering?.badgeStatus == null || rendering.icon == null) return null;
  return (
    <JobRowStacked
      status={rendering.badgeStatus}
      statusIcon={rendering.icon}
      statusLabel={rendering.verb ?? job.status}
      headline={job.title}
      jobId={job.job}
      {...(bare || job.handle === undefined ? {} : { handle: job.handle })}
      fields={fields}
      pulsing={job.status === "running"}
      onOpen={onOpen}
    />
  );
}

/** What the list says of one Job: what it waits on, where it landed, what it spent. */
function listFields(wave: WaveView, job: WaveJobView) {
  const waits = waitsOf(wave, job);
  const spent = waveSpentSaid(job);
  const tasks = waveTasksSaid(job);
  return [
    {
      label: "Waits on",
      value: waits.length === 0 ? "nothing" : waits.map((one) => one.title).join(", "),
      quiet: waits.length === 0,
    },
    ...(job.landed === undefined
      ? []
      : [{ label: "Pull request", value: job.landed === "merged" ? "merged" : "closed without merging" }]),
    ...(spent === undefined ? [] : [{ label: "Spent", value: spent }]),
    ...(tasks === undefined ? [] : [{ label: "Tasks done", value: tasks }]),
  ];
}

export type JobAnswerProps = Pick<
  WaveRegionProps,
  "board" | "questions" | "repositories" | "now" | "stale" | "acting" | "onAnswerJudge" | "onAnswerCommand"
> & { jobId: string };

/**
 * What one Job of the wave is asking you, answered where it is read: its
 * Judge refusal as the existing card, and a command the Manifest has not
 * cleared as the dock's own — allow for this Job, add the rule to the
 * repository's Manifest, or reject, the three `COMMAND_ANSWER` offers Fleet
 * sent (#1518, #1519). **Nothing where it asks nothing.**
 */
export function JobAnswer({
  jobId,
  board,
  questions,
  repositories,
  now,
  stale,
  acting,
  onAnswerJudge,
  onAnswerCommand,
}: JobAnswerProps) {
  const judge = questions.find(
    (one): one is JudgeAsking => one.kind === "judge" && one.job_id === jobId,
  );
  const commands = dockQuestionsOf(
    questions.filter((one) => one.kind === "command" && one.job_id === jobId),
    board,
    repositories,
    now,
    stale
      ? {}
      : {
          onAnswer: (question, answer) => {
            const named = answerNamed(answer);
            if (named !== undefined && question.kind === "command") {
              onAnswerCommand(question.job_id, question.waiting.call, named);
            }
          },
        },
  ).map((card) => (stale ? { ...card, note: NOT_LIVE } : card));
  if (judge === undefined && commands.length === 0) return null;
  return (
    <>
      {judge === undefined ? null : (
        <JudgeQuestion
          question={judge.question.question}
          expected={judge.question.expected}
          produced={judge.question.produced}
          consequence={judge.question.consequence}
          disabled={stale || acting}
          {...(stale ? { disabledNote: NOT_LIVE } : {})}
          onAnswer={(answer, note) => onAnswerJudge(jobId, judge.question.asked_at, answer, note)}
        />
      )}
      {commands.length === 0 ? null : <DockQuestions questions={commands} />}
    </>
  );
}

/**
 * What the Judge made of the live split: the model that read it, and each
 * criterion with its verdict. The verdict is the chip's hue, and named in its
 * tooltip.
 */
function Judged({ step, by }: { step: StepDetail | undefined; by: string | undefined }) {
  if (step === undefined || step.judged.length === 0) return null;
  // The last attempt's verdicts are the split being run; an earlier one was
  // the split a loop return replaced.
  const last = Math.max(...step.judged.map((one) => one.attempt));
  const judged = step.judged.filter((one) => one.attempt === last);
  return (
    <div className="armada-wave__judged" role="note" aria-label="How the Judge read the split">
      <Eyebrow>Judged</Eyebrow>
      {judged.map((one) => (
        <Tooltip key={one.criterion_id} label={one.verdict === "met" ? "Met" : "Not met"}>
          <FactChip named={one.verdict === "met" ? "met" : "not_met"}>{one.criterion_id}</FactChip>
        </Tooltip>
      ))}
      {by === undefined ? null : (
        <Tooltip label="The model that read the split">
          <span className="armada-wave__judged-by">{by}</span>
        </Tooltip>
      )}
    </div>
  );
}

export function WaveRegion({
  job,
  whole,
  draft,
  board,
  questions,
  view,
  onView,
  onOpenJob,
  planStep,
}: WaveRegionProps) {
  const wave = waveReadingOf(whole, draft, board);
  // Which pass the graph draws. **The live one unless a person pressed
  // another**, and this region's own: a pass pressed is a way of reading, not
  // something the Job holds.
  const live = wave?.rounds.find((one) => one.live)?.round ?? wave?.rounds.at(-1)?.round ?? 1;
  const [picked, setPicked] = useState<number | null>(null);
  const round = picked ?? live;
  const shown = useMemo(
    () => (wave === undefined ? undefined : { ...wave, jobs: wave.jobs.filter((one) => one.round === round) }),
    [wave, round],
  );
  // **Memoised, and the canvas is why.** `JobDetail` re-renders on every tick
  // of `now`; React Flow measures a node once and hides it until it has, so a
  // fresh node array each second left every card `visibility: hidden` and the
  // graph blank. The run is the same run until the wave itself changes.
  const opened = useCallback((jobId: string) => onOpenJob(jobId), [onOpenJob]);
  const run = useMemo(
    () => (shown === undefined ? undefined : waveRunOf(shown, { onOpen: opened })),
    [shown, opened],
  );
  if (wave === undefined || shown === undefined || run === undefined) return null;

  const byId = new Map(wave.jobs.map((one) => [one.job, one]));
  const mine = questions.filter((one) => one.kind !== "helm" && byId.has(one.job_id));
  const asking = new Set(mine.map((one) => (one.kind === "helm" ? "" : one.job_id)));
  const standing = waveStandingOf(wave, asking);
  const needing = new Set([...standing.blocked, ...standing.waiting].map((one) => one.job));
  // In the plan's own order, so a line and its node are found in the same place.
  const needs = wave.jobs.flatMap((one) => {
    const said = waitingOnSaid(one, mine, needing);
    return said === undefined ? [] : [{ job: one, said }];
  });

  const loop = loopSaid(whole, wave.rounds.length > 0);

  return (
    <section className="armada-wave" aria-label="The wave">
      <div className="armada-wave__head">
        <div className="armada-wave__passes">
          {wave.rounds.length > 1 ? (
            <Tabs
              items={wave.rounds.map((one) => ({
                id: String(one.round),
                label: `Wave ${String(one.round)} · ${one.says}`,
              }))}
              value={String(round)}
              onChange={(id) => setPicked(Number(id) === live ? null : Number(id))}
            />
          ) : wave.rounds[0] === undefined ? null : (
            <Eyebrow>{`Wave ${String(wave.rounds[0].round)} · ${wave.rounds[0].says}`}</Eyebrow>
          )}
          {loop === undefined ? null : <p className="armada-wave__loop">{loop}</p>}
        </div>
        <Tabs
          items={PLAN_VIEWS.map((one) => ({ id: one, label: PLAN_VIEW_LABEL[one] }))}
          value={view}
          onChange={(id) => onView(id as PlanView)}
        />
      </div>

      {needs.length === 0 ? null : (
        <section className="armada-wave__needs" aria-label="Needs you">
          <Eyebrow>Needs you</Eyebrow>
          <ActiveJobsList
            variant="panel"
            view="table"
            columns={["Waiting on"]}
            label="Needs you"
            selectable
          >
            {needs.map(({ job: one, said }) => (
              <WaveRow
                key={one.job}
                job={one}
                bare
                fields={[{ label: "Waiting on", value: said }]}
                onOpen={() => onOpenJob(one.job)}
              />
            ))}
          </ActiveJobsList>
        </section>
      )}

      {view === "graph" ? (
        <div className="armada-wave__canvas">
          <WaveCanvas
            nodes={run.nodes}
            edges={run.edges}
            label={`${job.title}, wave ${String(round)}, as the Jobs it dispatched and which waits on which`}
            opensOn={run.opensOn}
          />
        </div>
      ) : (
        <ActiveJobsList variant="panel" label={`Wave ${String(round)}, as a list`} selectable>
          {shown.jobs.map((one) => (
            <WaveRow
              key={one.job}
              job={one}
              fields={listFields(wave, one)}
              onOpen={() => onOpenJob(one.job)}
            />
          ))}
        </ActiveJobsList>
      )}

      {round === live ? <Judged step={planStep} by={wave.judged_by} /> : null}
    </section>
  );
}

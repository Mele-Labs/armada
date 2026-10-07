// What Fleet does with a step the Judge did not answer on, for the mock: asking
// again moves nothing while the Judge stays silent, and accepting advances the
// step and the Job carries on. Every other stopped Job is left as the scenario
// drew it, which is why each answers `undefined` for one it does not recognise.

import type { JobDetail, StepDetail } from "@armada/protocol";

/** The step `stuck` names, where the Job stopped on `gate_undecided`. */
function undecidedStep(whole: JobDetail): StepDetail | undefined {
  const stuck = whole.stuck;
  if (stuck?.stopped_by !== "gate_undecided") return undefined;
  return whole.steps.find((step) => step.step_id === stuck.step_id);
}

/** The step `stuck` names, where it stopped on a failed Check and Fleet offers the override. */
function checkStoppedStep(whole: JobDetail): StepDetail | undefined {
  const stuck = whole.stuck;
  if (stuck?.stopped_by !== "gate_failure" || !stuck.recourse.includes("override_verdict")) return undefined;
  const step = whole.steps.find((one) => one.step_id === stuck.step_id);
  return step?.check_runs.some((run) => run.outcome === "failed") ? step : undefined;
}

/** The Judge asked again and still did not answer: one more attempt, nothing else moved. */
export function askedAgain(whole: JobDetail, at: string): JobDetail | undefined {
  const asked = undecidedStep(whole);
  if (asked === undefined) return undefined;
  const attempt = (asked.attempts.at(-1)?.attempt ?? 0) + 1;
  const steps = whole.steps.map((step): StepDetail =>
    step !== asked
      ? step
      : {
          ...step,
          attempts: [...step.attempts, { attempt, outcome: "stopped", why: "gate_undecided", started_at: at }],
          verdicts: [...step.verdicts, { attempt, named: "failed", trigger: "gate_undecided" }],
          last_verdict: { attempt, named: "failed", trigger: "gate_undecided" },
          updated_at: at,
        },
  );
  return { ...whole, steps };
}

/** A person accepted the step, or overrode its failed Check: it advances, recorded as overridden, and the next one starts. */
export function accepted(whole: JobDetail, at: string): JobDetail | undefined {
  const held = undecidedStep(whole) ?? checkStoppedStep(whole);
  if (held === undefined) return undefined;
  const next = whole.steps.filter((step) => step.ordinal > held.ordinal).sort((a, b) => a.ordinal - b.ordinal)[0];
  const steps = whole.steps.map((step): StepDetail => {
    if (step === held) {
      return {
        ...step,
        state: "advanced",
        overridden: true,
        attempts: step.attempts.map((one, index, all) => (index === all.length - 1 ? { ...one, outcome: "advanced", ended_at: at } : one)),
        updated_at: at,
      };
    }
    if (step === next) {
      return {
        ...step,
        state: "running",
        attempts: [{ attempt: 1, outcome: "running", started_at: at }],
        entered_at: at,
        updated_at: at,
      };
    }
    return step;
  });
  const { reason: _stopped, ...job } = whole.job;
  const { stuck: _held, ...rest } = whole;
  return {
    ...rest,
    job: { ...job, status: next === undefined ? "completed_success" : "running", ...(next === undefined ? {} : { current_step_id: next.step_id }) },
    steps,
  };
}

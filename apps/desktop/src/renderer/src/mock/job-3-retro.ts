// The owner's Job 3 and its retro, for the Lessons page and the Record's
// Retro sheet. The Job is `featureAfterAgreeing`'s — the plan refused on
// `addresses_the_request` and the refusal agreed with — under Job 3's handle.
//
// **The retro is in Job 3's shape, written by hand**, each item where its fix
// lands (owner, 3 Oct 2026). Armada: the gate measured from local main, red
// ruled without confirming, Helm's acts through `curl`. Kit: the command T4 had
// to ask to allow. Manifest: desktop_test's fixed 15 s timeouts, and a docs
// edit setting off every Rust test. Job 2's retro predates `lands_in`. The words
// are stand-ins for what the retro call wrote; the shape is `crates/ipc/src/retro.rs`.

import type { JobRetro, Lesson } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { featureAfterAgreeing } from "./job-detail-refusal";
import { job2Landed } from "./job-2-landed";

const JOB_3_HANDLE = "3-retire-guides-8-and-20-add-validation-that";

/** Job 3, stopped on the Judge's refusal he agreed with. */
export function job3(): JobFixture {
  const base = featureAfterAgreeing();
  const job = { ...base.job, handle: JOB_3_HANDLE };
  if (base.watched.state !== "read") return { ...base, job };
  return {
    ...base,
    name: "Job 3, its retro written",
    job,
    watched: { ...base.watched, detail: { ...base.watched.detail, job } },
  };
}

const WRITTEN_3 = "2026-10-02T22:05:00.000Z";
const WRITTEN_2 = "2026-10-01T21:10:00.000Z";

/** Job 3's retro, as `GET /jobs/:job_id/retro` would answer it. */
export function job3Retro(jobId: string): JobRetro {
  return {
    job_id: jobId,
    state: "written",
    at: WRITTEN_3,
    items: [
      {
        who: "fleet",
        lands_in: "armada",
        statement:
          "The gate measured the diff from local main, so out_of_bounds on armada.yml blamed this Job for an upstream commit.",
        evidence: ["check:1"],
      },
      {
        who: "drone",
        lands_in: "armada",
        statement: "The Judge ruled addresses_the_request red without confirming what the request asked for.",
        evidence: ["not_met:1", "act:1", "restart:1"],
      },
      {
        who: "owner",
        lands_in: "armada",
        statement: "Helm restarted the step and re-ran the Checks with curl, so neither reads as your press.",
        evidence: ["act:2", "act:3"],
      },
      {
        who: "owner",
        lands_in: "kit",
        statement: "T4 had to ask you to allow a vitest command, and the step waited 19 minutes for the answer.",
        evidence: ["asked:1", "waited:1"],
      },
      {
        who: "drone",
        lands_in: "manifest",
        statement: "desktop_test's fixed 15 s timeouts ran out under load, so the Drone never saw it pass.",
        evidence: ["said:1", "note:1"],
      },
      {
        who: "owner",
        lands_in: "manifest",
        statement: "A docs edit set off every Rust test.",
        evidence: ["check:2"],
      },
    ],
    record: {
      failed_checks: [
        {
          cite: "check:1",
          at: "2026-10-02T21:12:40.000Z",
          step: "implement",
          attempt: 1,
          name: "out_of_bounds",
          run: "gate",
          expected: "Only the paths the plan names change",
          produced: "armada.yml changed; the line came in with 4e1c2a9 on main",
        },
        {
          cite: "check:2",
          at: "2026-10-02T21:31:00.000Z",
          step: "implement",
          attempt: 2,
          name: "rust_test",
          run: "gate",
          produced: "every Rust test ran for a change to docs/concepts/retro.md alone",
        },
      ],
      not_met: [
        {
          cite: "not_met:1",
          step: "plan",
          attempt: 1,
          criterion: "addresses_the_request",
          expected: "Tasks to retire guides 8 and 20 and add the rule",
          produced: "T4 adds documentation updates to design-system.md",
        },
      ],
      restarts: [
        { cite: "restart:1", at: "2026-10-01T20:33:10.000Z", actor: "owner", via: "bridge", moved: "restart_step" },
      ],
      asked: [
        {
          cite: "asked:1",
          asked_at: "2026-10-02T20:51:00.000Z",
          step: "implement",
          about: "Allow pnpm --dir packages/screens exec vitest run?",
          answered_at: "2026-10-02T21:10:00.000Z",
          answer: "Allow",
          waited_ms: 1_140_000,
        },
      ],
      waited: [
        {
          cite: "waited:1",
          status: "awaiting_human",
          from: "2026-10-02T20:51:00.000Z",
          until: "2026-10-02T21:10:00.000Z",
          ms: 1_140_000,
        },
      ],
      acts: [
        {
          cite: "act:1",
          at: "2026-10-01T20:31:04.000Z",
          actor: "owner",
          via: "bridge",
          moved: "answer_judge_question",
          said: "Agree with the refusal",
        },
        { cite: "act:2", at: "2026-10-02T21:20:00.000Z", actor: "helm", via: "http", moved: "restart_step" },
        { cite: "act:3", at: "2026-10-02T21:22:30.000Z", actor: "helm", via: "http", moved: "rerun_checks" },
      ],
      said_after: [
        {
          cite: "said:1",
          at: "2026-10-02T21:40:00.000Z",
          step: "implement",
          said: "I never saw desktop_test pass; its 15 s timeouts ran out before the tests did.",
        },
      ],
      notes: [
        { cite: "note:1", at: "2026-10-02T21:39:00.000Z", step: "implement", said: "desktop_test timed out under load." },
      ],
    },
    annotations: [
      {
        id: "20261002-213000-k3f9",
        at: "2026-10-02T21:30:00.000Z",
        text: "Why did the restart show twice in the Record?",
        screen: "Overview",
      },
    ],
  };
}

/** Job 2's retro: written, and shorter. */
export function job2Retro(jobId: string): JobRetro {
  return {
    job_id: jobId,
    state: "written",
    at: WRITTEN_2,
    items: [
      {
        who: "owner",
        statement: "The Judge's question waited in the dock while the plan was read twice.",
        evidence: ["waited:1"],
      },
    ],
    record: {
      waited: [
        {
          cite: "waited:1",
          status: "awaiting_review",
          from: "2026-10-01T20:26:21.196Z",
          until: "2026-10-01T20:31:04.000Z",
          ms: 282_804,
        },
      ],
    },
  };
}

/** The Lessons listing over both, newest retro first, as `GET /lessons` serves it. */
export function lessonsOver(three: JobFixture, two: JobFixture): Lesson[] {
  const of = (fixture: JobFixture, retro: JobRetro): Lesson[] =>
    (retro.items ?? []).map((item) => ({
      job_id: fixture.job.id,
      handle: fixture.job.handle,
      at: retro.at ?? "",
      ...item,
    }));
  return [...of(three, job3Retro(three.job.id)), ...of(two, job2Retro(two.job.id))];
}

/** Job 3 and Job 2, each with its retro, and the Lessons page over both. */
export function retroFixtures(): { fixtures: JobFixture[]; retros: Record<string, JobRetro>; lessons: Lesson[] } {
  const three = job3();
  const two = job2Landed();
  return {
    fixtures: [three, two],
    retros: { [three.job.id]: job3Retro(three.job.id), [two.job.id]: job2Retro(two.job.id) },
    lessons: lessonsOver(three, two),
  };
}

// The owner's Job 3 and its retro, for the Lessons page and the Record's
// Retro sheet. The Job is `featureAfterAgreeing`'s — the plan refused on
// `addresses_the_request` and the refusal agreed with — under Job 3's handle.
//
// **The retro is Job 3's real story, written by hand as a person would write
// it** (owner, 4 Oct 2026): a title, what happened and the fix, each item where
// its fix lands. Two are Fleet's own mistakes (the gate measured from a stale
// local main, and ran a step's Checks all at once), one is a command a Drone had
// to ask to be allowed (Kit), one is a docs edit that ran every Rust test
// (Manifest). Job 2's retro predates all of it: its one item carries a
// `statement` and nothing else. The words are stand-ins for what the retro call
// wrote; the shape is `crates/ipc/src/retro.rs`.

import type { JobRetro, Lesson, RetroItem } from "@armada/protocol";
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

/** Job 3's four items, in the order the retro wrote them. */
const JOB_3_ITEMS: RetroItem[] = [
  {
    id: "01M2LESSON3STALEMAIN",
    who: "fleet",
    lands_in: "armada",
    statement:
      "The gate compared the step against a stale local main, so two commits that edited armada.yml counted as the Drone's work.",
    title: "The gate blamed the Drone for Fleet's own mistake",
    what: "It compared the step against a local main two commits behind origin, so two commits that edited armada.yml counted as the Drone's work.",
    fix: "Compare against origin/main, where the branch is cut from.",
    evidence: ["check:1"],
  },
  {
    id: "01M2LESSON3ALLATONCE",
    who: "fleet",
    lands_in: "armada",
    statement: "The gate ran a step's Checks all at once, which slowed the browser tests 5 to 8 times until they timed out.",
    title: "Browser tests timed out under the gate",
    what: "The gate ran a step's Checks all at once, which slowed the browser tests 5 to 8 times.",
    fix: "Run a step's Checks one at a time.",
    evidence: ["check:3", "said:1", "note:1"],
  },
  {
    id: "01M2LESSON3GREPASKED",
    who: "drone",
    lands_in: "kit",
    statement: "A Drone asked to run grep on a check log and waited for the owner to allow it.",
    title: "A Drone had to wait for grep to be allowed",
    what: "It asked to run grep on a check log, and the step waited until you allowed it.",
    fix: "Add grep on .armada/checks to the allowlist.",
    evidence: ["refusal:1", "asked:1", "waited:1"],
  },
  {
    id: "01M2LESSON3DOCSTESTS",
    who: "fleet",
    lands_in: "manifest",
    statement: "A docs edit set off all 4211 Rust tests and a 7.5 minute compile.",
    title: "A docs edit ran every Rust test",
    what: "One docs edit set off all 4211 Rust tests and a 7.5 minute compile.",
    fix: "Run only xtask's tests when only apps/ or packages/ change.",
    evidence: ["check:2"],
  },
];

/** Job 3's retro, as `GET /jobs/:job_id/retro` would answer it. */
export function job3Retro(jobId: string): JobRetro {
  return {
    job_id: jobId,
    state: "written",
    at: WRITTEN_3,
    items: JOB_3_ITEMS,
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
        {
          cite: "check:3",
          at: "2026-10-02T21:38:00.000Z",
          step: "implement",
          attempt: 2,
          name: "desktop_test",
          run: "gate",
          produced: "15 s timeouts ran out while the gate ran every other Check",
        },
      ],
      refusals: [
        {
          cite: "refusal:1",
          at: "2026-10-02T20:50:30.000Z",
          step: "implement",
          tool: "Bash",
          tried: "grep -n timed .armada/checks/desktop_test.log",
          because: "not on the allowlist",
        },
      ],
      asked: [
        {
          cite: "asked:1",
          asked_at: "2026-10-02T20:51:00.000Z",
          step: "implement",
          about: "Allow grep on .armada/checks?",
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

/** Job 2's retro: written before an item had a title, a what or a fix, so its one item has a statement alone. */
export function job2Retro(jobId: string): JobRetro {
  return {
    job_id: jobId,
    state: "written",
    at: WRITTEN_2,
    items: [
      {
        id: "01M2LESSON2DOCKWAIT",
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

/** The Lessons listing over both, newest retro first, as `GET /lessons` serves it: every item still open. */
export function lessonsOver(three: JobFixture, two: JobFixture): Lesson[] {
  const of = (fixture: JobFixture, retro: JobRetro): Lesson[] =>
    (retro.items ?? []).map((item) => ({
      job_id: fixture.job.id,
      handle: fixture.job.handle,
      at: retro.at ?? "",
      state: "open",
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

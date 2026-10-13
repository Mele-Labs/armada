// The Cockpit with four Jobs that went wrong, one of each kind a call can say: a Check failed, a Drone
// gone quiet, a ruling that failed, and a Job in the same files as another. Each card shows the failure
// and offers the fixes, so the owner need not open the Job. The walk `jobCallFixes` plays it.

import { featureRunning } from "@armada/jobs/fake";
import type { CallView } from "@armada/jobs/draft/calls";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const OWNER = repository().manifest!.id;

/** A Job the Drone is asking about: running, flagged as asking, no longer cleared, and the picked repository's. */
function asking(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, asking: true, owner_manifest_id: OWNER };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, asking: true, owner_manifest_id: OWNER };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const failed = asking(asRow(featureRunning(), 61, "retry-backoff", "Cap the retry backoff"));
const quiet = asking(asRow(featureRunning(), 62, "pin-store-clock", "Pin the store clock in every test"));
const ruled = asking(asRow(featureRunning(), 63, "retire-sleeps", "Retire sleep calls in the store tests"));
const clashing = asking(asRow(featureRunning(), 64, "reconnect-wait", "Shorten the reconnect wait"));

const calls: Record<string, CallView> = {
  [failed.job.id]: {
    request: "Cap the retry backoff in the store client at 30 seconds.",
    issues: [
      {
        key: "i1",
        of: "check",
        name: "store",
        text: "The store Check failed",
        said: "Check failed",
        unfinished: 2,
        failing: ["store::retry::caps_at_thirty_seconds", "store::retry::resets_after_success", "store::retry::jitter_stays_inside_the_cap"],
        output: [
          "running 14 tests",
          "test retry::doubles_each_attempt ... ok",
          "test retry::caps_at_thirty_seconds ... FAILED",
          "test retry::resets_after_success ... FAILED",
          "test retry::jitter_stays_inside_the_cap ... FAILED",
          "",
          "failures:",
          "",
          "---- retry::caps_at_thirty_seconds ----",
          "thread 'retry::caps_at_thirty_seconds' panicked at crates/store/src/retry.rs:88:9:",
          "assertion `left == right` failed",
          "  left: 64s",
          " right: 30s",
          "",
          "test result: FAILED. 11 passed; 3 failed; 0 ignored",
        ],
      },
    ],
  },
  [quiet.job.id]: {
    request: "Pin the clock in every store test.",
    issues: [
      {
        key: "i1",
        of: "drone",
        name: "Drone on Implement",
        text: "Gone quiet",
        said: "Drone stuck",
        quiet: {
          minutes: 47,
          last: ["Read crates/store/tests/clock.rs", "Edit crates/store/tests/clock.rs", "cargo test -p store clock", "cargo test -p store clock"],
        },
      },
    ],
  },
  [ruled.job.id]: {
    request: "Retire the sleep calls in the store tests.",
    issues: [
      {
        key: "i1",
        of: "judge",
        name: "Judge on Review",
        text: "The change was ruled failed",
        said: "Failed ruling",
        ruling: {
          attempt: "2 of 3",
          trigger: "Review",
          reason: "Two tests still sleep for a second, so they race the clock the change was meant to pin.",
        },
      },
    ],
  },
  [clashing.job.id]: {
    request: "Cut the first reconnect wait from 30s to 5s.",
    issues: [
      {
        key: "i1",
        of: "clash",
        name: "Overlap",
        text: "Another Job is in the same files",
        said: "Overlap",
        clash: {
          what: "Pin the store clock in every test is writing the same files.",
          paths: ["crates/store/src/reconnect.rs", "crates/store/tests/clock.rs"],
          other: { jobId: quiet.job.id, title: "Pin the store clock in every test" },
        },
      },
    ],
  },
};

/** Fleet takes a moment over Message and Retry step, as it does over a Check fix, so a card shows its pending state. */
const BEAT_MS = 350;
const beat = () => new Promise<{ ok: true }>((resolve) => setTimeout(() => resolve({ ok: true }), BEAT_MS));

function build(): Scenario {
  const base = holding("job-call-fixes", "The Cockpit with a Job whose Check failed, one whose Drone went quiet, one whose ruling failed and one in another Job's files", [failed, quiet, ruled, clashing]);
  return { ...base, state: { ...base.state, repository: repository().root, jobs: [failed.job, quiet.job, ruled.job, clashing.job] }, draft: { calls }, behaves: () => ({ redirectDrone: beat, restartStep: beat }) };
}

export const s211JobCallFixes: Scenario = build();

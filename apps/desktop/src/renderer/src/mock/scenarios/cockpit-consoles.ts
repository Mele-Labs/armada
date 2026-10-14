// The Cockpit as a ship's console: the calls of `job-call-fixes` plus a Session waiting on a question in
// words, and one more Job whose Drone goes quiet a moment after the walk leaves the Cockpit, so a call
// arrives with the owner somewhere else. The walk `cockpitConsoles` plays it.

import { featureRunning } from "@armada/jobs/fake";
import type { CallView } from "@armada/jobs/draft/calls";
import { repository } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow } from "../holding";
import type { Scenario } from "../moment";
import { s211JobCallFixes } from "./job-call-fixes";
import { s210SessionCallComposer } from "./session-call-composer";

const OWNER = repository().manifest!.id;

/** A Job the Drone is asking about: running, flagged as asking, no longer cleared, and the picked repository's. */
function asking(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, asking: true, owner_manifest_id: OWNER };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  const { reclaimed_at: _also, ...detailJob } = { ...fixture.watched.detail.job, asking: true, owner_manifest_id: OWNER };
  return { ...fixture, job, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: detailJob } } };
}

const arriving = asking(asRow(featureRunning(), 65, "reconnect-ceiling", "Raise the reconnect ceiling"));

const view: CallView = {
  request: "Raise the reconnect ceiling from 60s to 120s.",
  issues: [
    {
      key: "i1",
      of: "drone",
      name: "Drone on Implement",
      text: "Gone quiet",
      said: "Drone stuck",
      quiet: { minutes: 23, last: ["Read crates/store/src/reconnect.rs", "Edit crates/store/src/reconnect.rs", "cargo test -p store reconnect"] },
    },
  ],
};

function build(): Scenario {
  const base = s211JobCallFixes;
  return {
    ...base,
    name: "cockpit-consoles",
    says: "The Cockpit with its acts in consoles beside the card, a call that arrives while he is on Active, and a Session's question in words",
    reads: { ...base.reads, [arriving.job.id]: arriving },
    // The first moment is a beat with nothing in it: a call is only news once Bridge has settled on what it holds.
    later: [{}, { jobs: [...base.state.jobs, arriving.job] }],
    draft: { ...base.draft, calls: { ...base.draft!.calls, [arriving.job.id]: view }, sessions: s210SessionCallComposer.draft!.sessions! },
  };
}

export const s212CockpitConsoles: Scenario = build();

// A Job started by another Job whose starter has ended and left the list: the Job stands as a plain row with no link and no connector,
// and with All chosen the starter is listed again and the link returns. The walk `orphanLink` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const STARTER = "01ORPHANSTARTERAAAAAAAAAAA";
const CHILD = "01ORPHANCHILDAAAAAAAAAAAAA";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const records = () => [
  hosted("01ORPHANABOVEAAAAAAAAAAAAA", { title: "Draft the release notes", last_turn_at: minutesAgo(1), hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted(CHILD, { title: "Trim the retry loop", last_turn_at: minutesAgo(2), attachments: [held("started_by", STARTER, {}, "spent")], hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted(STARTER, { title: "Plan the retry work", state: "ended", last_seen_at: minutesAgo(40), last_turn_at: minutesAgo(40), attachments: [held("started_to", CHILD, {}, "spent")] }),
];

const fleet = () => new FakeSessionsFleet(records(), {});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = fleet().scenario(board);
  return {
    ...served,
    name: "orphan-link",
    says: "The Cockpit's list with a Job whose starter has ended",
    behaves: (handle) => fleet().scenario(board).behaves!(handle),
  };
}

export const orphanLinkList: Scenario = build();

// A Job in one repository that started a Job in its own repository and one in another, on All with both served: the one in its
// own repository stands under it with the line, and the one in the other stands in its own repository's group as a plain row with
// the link glyph. The walk `cross-repo-link` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

const PARENT = "01CROSSPARENTAAAAAAAAAAAAA";
const AWAY = "01CROSSAWAYBBBBBBBBBBBBBBB";
const HOME = "01CROSSHOMECCCCCCCCCCCCCCC";

const now = () => new Date().toISOString();
const working = { turn: { state: "working" as const }, mode: "auto" as const, running: true };
const storefront = { manifest_id: "storefront", cwd: "/Users/user/code/storefront" };

const records = () => [
  hosted(PARENT, { title: "Plan the checkout rework", last_turn_at: now(), attachments: [held("started_to", AWAY, {}, "spent"), held("started_to", HOME, {}, "spent")], hosted: working }),
  hosted("01CROSSARMADADDDDDDDDDDDDD", { title: "Draft the migration notes", last_turn_at: now(), hosted: working }),
  hosted(HOME, { title: "Trim the retry loop", last_turn_at: now(), attachments: [held("started_by", PARENT, {}, "spent")], hosted: working }),
  hosted("01CROSSSTOREEEEEEEEEEEEEEE", { ...storefront, title: "Lay out the invoice PDF", last_turn_at: now(), hosted: working }),
  hosted(AWAY, { ...storefront, title: "Round the cart totals", last_turn_at: now(), attachments: [held("started_by", PARENT, {}, "spent")], hosted: working }),
];

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA, STOREFRONT] });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: "cross-repo-link",
    says: "The Cockpit's list on All with a Job that started one in its own repository and one in another",
    behaves: (handle) => new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle),
  };
}

export const crossRepoLinkList: Scenario = build();

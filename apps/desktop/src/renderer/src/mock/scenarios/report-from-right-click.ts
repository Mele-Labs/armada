// A right press anywhere in Bridge, with the annotation layer off: a menu with Report, and a Job's row
// carries it among its own. The walk `report-from-right-click` plays it. The layer is the mock's own, off (`../annotating.tsx`).

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const sessions = () => [hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] })];

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(sessions()).scenario(board);
  return {
    ...served,
    name: "report-from-right-click",
    says: "A right press opens a menu with Report on anything, and Report opens a note on what was pressed",
    // A Fleet of its own for each window, so a walk told twice starts from the same Job.
    behaves: (handle) => new FakeSessionsFleet(sessions()).scenario(board).behaves!(handle),
  };
}

export const s312ReportFromRightClick: Scenario = build();

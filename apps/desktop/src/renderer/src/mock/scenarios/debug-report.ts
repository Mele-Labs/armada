// Copy debug report, on Doctor and in the palette: the sheet reads, shows the text it will copy, and Copy copies it.
// `debug-report` answers the first opening in full and the second as a Fleet that did not answer;
// `debug-report-reading` never answers, so the sheet holds its placeholder. The claims are in `walks/debug-report.ts`.

import { manifest, repository } from "@armada/screens/src/fixtures/repository";

import { connected, type Scenario } from "../moment";
import { REPORT, REPORT_FLEET_SILENT } from "../debug-report-fixtures";

// The same repository the report names, in place of the app's own.
const WEB_APP = { ...repository(), root: "/Users/user/web-app", manifest: { ...manifest(), repository: "web-app", path: "web-app/armada.yml" } };

const state = {
  ...connected([], [WEB_APP]),
  repository: WEB_APP.root,
  health: {
    state: "read" as const,
    health: {
      probes: [
        { module: "Fleet", outcome: "pass" as const, detail: "answering" },
        { module: "SQLite", outcome: "pass" as const, detail: "opens" },
        { module: "Manifest", outcome: "pass" as const, detail: "re-read and adopted" },
      ],
      not_probed: [],
      helm_action_authority: "acting" as const,
    },
  },
};

export const debugReport: Scenario = {
  name: "debug-report",
  says: "Doctor and the palette offer Copy debug report: a sheet with the text it will copy, then the same with Fleet not answering",
  state,
  draft: { debugReport: [{ text: REPORT }, { text: REPORT_FLEET_SILENT }] },
};

export const debugReportReading: Scenario = {
  name: "debug-report-reading",
  says: "Copy debug report while Fleet has not answered yet: the sheet holds its placeholder",
  state,
  draft: { debugReport: [{ text: REPORT, after: Infinity }] },
};

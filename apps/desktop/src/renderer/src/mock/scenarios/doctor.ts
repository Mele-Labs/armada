// Doctor with something to say: one Manifest that would not re-read, the machine short of headroom,
// and the probes `adapters` and `config` own still unbuilt, as Fleet names them. Open it from the
// Fleet panel's Doctor row.

import { repository } from "@armada/screens/src/fixtures/build/base";

import { connected, type Scenario } from "../moment";

const state = connected([], [], [repository()]);

export const s200Doctor: Scenario = {
  name: "doctor",
  says: "Doctor: a Manifest that would not re-read, System stats short of headroom, and the probes nobody runs yet",
  state: {
    ...state,
    repository: repository().root,
    health: {
      state: "read",
      health: {
        probes: [
          { module: "Fleet", outcome: "pass", detail: "answering — this process is run 01M1CNPKTV0018H2M1CXDNBK0R" },
          { module: "SQLite", outcome: "pass", detail: "opens; 42 Jobs read back" },
          { module: "Manifest", outcome: "pass", detail: "/Users/user/armada/armada.yml, re-read and adopted" },
          { module: "Manifest", outcome: "fail", detail: "/Users/user/ledger/armada.yml would not re-read: checks.test names no command" },
          { module: "System stats", outcome: "warn", detail: "cpu 64% in use, memory 91% in use, 18874368 KiB free on the volume" },
        ],
        not_probed: [
          { owner: "adapters", because: "it owns talking to anything outside Armada, and Doctor's four probes of those live there. None is built" },
          { owner: "config", because: "it reads and validates its own files, and Doctor's two probes of those live there. Neither is built" },
          { owner: "Bridge", because: "the Armada API row is not a probe: it is a client's own connection state" },
        ],
        helm_action_authority: "acting",
      },
    },
  },
  reads: {},
};

// A pull request the forge refused: #1899's page comes back as Fleet's words, the forge's reason and the step that fixes it, so
// the panel says that and not only that the forge did not answer. The step it names is a command the panel can run: the fake
// Fleet's run prints one line and exits. Every other read is `shared-side-panel`'s. The walk `pr-panel-says-why-refused` plays it.

import type { SessionRunMessage } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s263SharedSidePanel as base } from "./shared-side-panel";

const REFUSED = 1899;
const REFUSAL = "GitHub refused the token gh signed in with. Sign in again with `gh auth refresh -s repo`, authorize it for the organization's SSO, then open it again.";

/** A Fleet that runs what it is given: it prints a line, then ends. Output is sent as main sends it, by run id. */
function fakeRun(): NonNullable<ReturnType<NonNullable<NonNullable<Scenario["draft"]>["sharedPanel"]>>["terminalRun"]> {
  const hearers = new Set<(runId: string, message: SessionRunMessage) => void>();
  const ok = async () => ({ ok: true, value: { run_id: "run-gh" } }) as const;
  return {
    startTerminalRun: async (command: string) => {
      const send = (message: SessionRunMessage) => hearers.forEach((one) => one("run-gh", message));
      setTimeout(() => send({ kind: "output", text: `$ ${command}\r\nLogged in to github.com as ines\r\n` }), 200);
      setTimeout(() => send({ kind: "exited", code: 0 }), 500);
      return ok();
    },
    writeSessionRun: ok,
    resizeSessionRun: ok,
    stopSessionRun: ok,
    onSessionRunMessage: (on) => {
      hearers.add(on);
      return () => void hearers.delete(on);
    },
  };
}

export const s264PrPanelRefused: Scenario = {
  ...base,
  name: "pr-panel-refused",
  says: "A pull request the forge refused, with Fleet's reason and what to do",
  draft: {
    ...base.draft,
    sharedPanel: (...args) => {
      const reads = base.draft!.sharedPanel!(...args);
      return { ...reads, terminalRun: fakeRun(), pullRequest: (item) => (item.number === REFUSED ? Promise.resolve({ refused: REFUSAL, root: "/code/armada" }) : reads.pullRequest(item)) };
    },
  },
};

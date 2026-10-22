// A Session's background tasks, as the `ledger-in-cockpit` scenario holds them: two shell
// commands and a monitor still running, and two ended, one that exited 0 and one that was stopped. Here, not under
// `scenarios/`, because every file there is one scenario.

import type { Attachment, SessionRecord } from "@armada/protocol";
import { attachmentsOfRecord } from "@armada/screens/src/sessions-wire";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A background task as the wire carries it, which the Session's ledger reads through the same mapping a real Fleet's is. */
const task = (target: string, form: "command" | "monitor", said: string, command: string, since: number, rest: { state?: Attachment["state"]; status?: string; exit?: string; ended?: number } = {}): Attachment => ({
  kind: "background",
  manifest_id: "armada",
  target,
  state: rest.state ?? "standing",
  detail: { form, task: said, command, output: `/tmp/armada/tasks/${target}.output`, ...(rest.status === undefined ? {} : { status: rest.status }), ...(rest.exit === undefined ? {} : { exit: rest.exit }) },
  since: minutesAgo(since),
  changed_at: minutesAgo(rest.ended ?? since),
});

const wire = [
  task("serve-mock", "command", "Serve the Bridge mock from slot 6", "pnpm --filter @armada/desktop exec vite --port 5176 --host 127.0.0.1", 18),
  task("watch-tests", "command", "Rerun the session tests on save", "pnpm --filter @armada/desktop exec vitest watch src/renderer/src/sessions", 12),
  task("watch-ci", "monitor", "Watch the CI run for #1862", "gh run watch 9321177 --exit-status", 7),
  task("build-renderer", "command", "Build the Bridge renderer", "pnpm --filter @armada/desktop build", 41, { state: "spent", status: "completed", exit: "0", ended: 40 }),
  task("tail-fleet", "command", "Follow the Fleet log", "tail -f ~/Library/Logs/Armada/fleet.log", 33, { state: "spent", status: "killed", ended: 25 }),
];

/** The Session's background tasks as its ledger reads them. */
export const backgroundAttachments = attachmentsOfRecord({ attachments: wire } as SessionRecord, { sketches: [] });

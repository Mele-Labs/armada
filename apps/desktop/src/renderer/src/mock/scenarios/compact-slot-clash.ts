// The compact Session holding two bays, in write clashes with two other Sessions: one over a file, one over eight. Fleet tells both agents,
// so a clash lights the Bays key and shows as the notice under the header, and raises no call in the thread. The walk `compactSlotClash` plays it.

import { compactSession } from "../compact-session";
import type { ClashFileRead } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";

const base = compactSession("compact-slot-clash", "A Session holding two bays, in write clashes with two other Sessions, with no call for them in the thread");
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** A side's change to a file: an import, and a hunk at line 41 that both sides change, so it collides. The other side's first hunk is further down. */
function patchOf(path: string, ours: boolean): string {
  const head = `diff --git a/${path} b/${path}\n--- a/${path}\n+++ b/${path}\n`;
  return ours
    ? `${head}@@ -12,3 +12,4 @@ use crate::pool::Pool;\n use crate::slots::Slot;\n+use crate::clock::Backoff;\n \n@@ -41,4 +42,6 @@ impl Lease {\n     pub fn take(&self, pool: &Pool) -> Result<Slot, Contended> {\n-        pool.take_free()\n+        let backoff = Backoff::new(self.tries);\n+        pool.take_free().or_else(|_| backoff.retry(|| pool.take_free()))\n     }\n`
    : `${head}@@ -40,5 +40,6 @@ impl Lease {\n \n     pub fn take(&self, pool: &Pool) -> Result<Slot, Contended> {\n-        pool.take_free()\n+        let taken = pool.take_free();\n+        self.audit.record(&taken);\n+        taken\n     }\n@@ -88,2 +89,3 @@ impl Drop for Lease {\n     fn drop(&mut self) {\n+        self.audit.release(self.slot);\n`;
}

export const s274CompactSlotClash: Scenario = {
  ...base,
  // Its second bay is the one the compact pool leaves free.
  held: {
    ...base.held!,
    slots: (base.held!.slots ?? []).map((one) =>
      one.slot === 4 ? { ...one, held: { state: "hosted_session", session_id: "compact-1", session_title: "Retry the slot lease" }, branch: "armada/lease-retry-wire", since: ago(12), behind: 2 } : one,
    ),
  },
  draft: {
    ...base.draft,
    sessions: (board, more) => ({
      ...base.draft!.sessions!(board, more),
      clashFile: async (_id, _other, path): Promise<ClashFileRead> => ({ ok: true, ours: patchOf(path, true), theirs: patchOf(path, false) }),
    }),
    clashes: [
      {
        one: { kind: "session", id: "compact-1", label: "Retry the slot lease" },
        other: { kind: "session", id: "compact-3", label: "Name the store migrations" },
        paths: ["crates/ipc/src/lib.rs"],
        since: ago(5),
      },
      {
        one: { kind: "session", id: "compact-4", label: "Rotate the audit log" },
        other: { kind: "session", id: "compact-1", label: "Retry the slot lease" },
        paths: [
          "crates/fleet/src/audit.rs",
          "crates/fleet/src/lease.rs",
          "crates/fleet/src/pool.rs",
          "crates/fleet/src/slots.rs",
          "crates/fleet/src/tests/audit.rs",
          "crates/fleet/src/tests/lease.rs",
          "crates/store/src/audit.rs",
          "docs/concepts/fleet.md",
        ],
        since: ago(9),
      },
    ],
  },
};

// The Commander's thread as the Session thread draws it. **Words and marks are written here**, so
// the component draws no sentence of its own: an act's glyph and the name its tooltip says, and the
// one line of facts it leaves in the thread.

import { ArrowUpToLine, RotateCw, Split } from "lucide-react";
import type { SessionThreadRow } from "@armada/components";

import type { CommanderAct, CommanderRow } from "./draft/commander";

function actOf(act: CommanderAct): { icon: typeof Split; said: string; text: string } {
  switch (act.kind) {
    case "raise_cap":
      return { icon: ArrowUpToLine, said: "Raised cap", text: `${act.cap === "turns" ? "Turn cap" : "Cost cap"} ${act.from} → ${act.to}` };
    case "split":
      return { icon: Split, said: "Split task", text: `${act.task} split into ${act.into.join(", ")}` };
    case "restart":
      return { icon: RotateCw, said: "Restarted task", text: `${act.target} restarted` };
  }
}

export function commanderThreadOf(rows: readonly CommanderRow[]): SessionThreadRow[] {
  return rows.map((row): SessionThreadRow => {
    switch (row.kind) {
      case "woke":
        return { id: row.id, at: row.at, kind: "woke", why: row.why };
      case "you":
        return { id: row.id, at: row.at, kind: "message", from: "you", text: row.text };
      case "said":
        return { id: row.id, at: row.at, kind: "message", from: "agent", text: row.text };
      case "read":
        return { id: row.id, at: row.at, kind: "tool", text: row.text };
      case "act":
        return { id: row.id, at: row.at, kind: "act", ...actOf(row.act) };
    }
  });
}

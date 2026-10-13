// The Commander's thread as the Session thread draws it, from what its socket carried. **Words and
// marks are written here**, so the component draws no sentence of its own: an act's glyph and the
// name its tooltip says, and the one line of facts it leaves in the thread.
//
// **Only what the wire says.** A row is the session's own: a call carries its tool and one line of
// argument, so an act's row names the act and no figure, and the plan's own marks say which task it
// touched. A call is an `act` row where it is one of the three the owner has seen drawn as one, and
// a quiet `tool` row otherwise; nothing it said or did is read as more than that.

import { ArrowUpToLine, RotateCw } from "lucide-react";
import type { SessionThreadRow } from "@armada/components";
import type { CommanderState } from "@armada/components";
import type { CommanderThread, CommanderThreadItem, JobDetail } from "@armada/protocol";
import { clock } from "@armada/screens/src/duration";

/** A tool as its session names it, `mcp__<server>__raise_turn_cap`, without the server. */
const bare = (tool: string): string => tool.replace(/^mcp__.*?__/, "");

const ACTS: Readonly<Record<string, { icon: typeof RotateCw; said: string; text: string }>> = {
  raise_turn_cap: { icon: ArrowUpToLine, said: "Raised cap", text: "Turn cap" },
  raise_cost_cap: { icon: ArrowUpToLine, said: "Raised cap", text: "Cost cap" },
  restart_task: { icon: RotateCw, said: "Restarted task", text: "Task restarted" },
};

/**
 * The thread's rows. `jobId` is dropped from a call's argument: every call a Commander makes
 * names its own Job, and the row would say nothing else.
 */
export function commanderThreadOf(items: readonly CommanderThreadItem[], jobId: string): SessionThreadRow[] {
  return items.flatMap((item): SessionThreadRow[] => {
    switch (item.kind) {
      case "woke":
        return [{ id: item.id, at: clock(item.ts), kind: "woke", why: item.why }];
      case "asked":
        return [{ id: item.id, at: clock(item.ts), kind: "message", from: "you", text: item.text }];
      case "fresh":
        // Helm's sentence for the same fact (`helm-thread.ts`), in the same lower case.
        return [{ id: item.id, at: clock(item.ts), kind: "message", from: "agent", text: "the stored session could not be resumed, so this reply starts a new one" }];
      case "unanswered":
        // Fleet's sentence whole and unframed.
        return [{ id: item.id, at: clock(item.ts), kind: "message", from: "agent", text: item.why }];
      case "row": {
        const { ts } = item.turn;
        const saw = item.turn.saw;
        const at = clock(ts);
        // The brief Fleet sent and the harness replayed is not the Commander's to be read as saying.
        if (saw.event === "said") return item.turn.by === "drone" ? [{ id: item.id, at, kind: "message", from: "agent", text: saw.text }] : [];
        if (saw.event !== "called") return [];
        const name = bare(saw.tool);
        const act = ACTS[name];
        if (act !== undefined) return [{ id: item.id, at, kind: "act", ...act }];
        const detail = saw.detail === jobId ? "" : saw.detail;
        return [{ id: item.id, at, kind: "tool", text: `${name} ${detail}`.trim() }];
      }
    }
  });
}

/**
 * What the Commander panel draws for the Job open, or nothing where its thread is another Job's or
 * none is open. **A card out is `waiting`, whatever else the Commander is doing**: it is the
 * Commander that cannot go on without the owner.
 */
export function commanderViewOf(
  thread: CommanderThread | undefined,
  jobId: string,
  whole: Pick<JobDetail, "commander_card"> | null,
): { state: CommanderState; rows: SessionThreadRow[] } | undefined {
  if (thread === undefined || thread.state === "none" || thread.jobId !== jobId) return undefined;
  const items = thread.state === "opening" ? [] : thread.items;
  const replying = thread.state === "opening" ? false : thread.replying;
  return {
    state: whole?.commander_card !== undefined ? "waiting" : replying ? "working" : "idle",
    rows: commanderThreadOf(items, jobId),
  };
}

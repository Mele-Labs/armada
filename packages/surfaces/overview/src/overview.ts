// What the left column's Fleet panel reads, from what Fleet already serves. Overview's
// own tile band read the same five once (#919); Overview 27 (#1091) replaced the band with the
// summary strip and left these readings here — `left-column.ts` is their only caller now.
//
// **Fleet and Doctor read the machine.** A pick narrows none of them —
// `docs/concepts/fleet.md` names what stays Fleet-wide.

import type { Connection } from "@armada/protocol";
import type { ReactNode } from "react";
import { statementOf } from "@armada/shell/src/fleet";
import { said } from "@armada/screens/src/copy";
import type { FleetHealth } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";

/** One of the state machine's own hues, or none — never a colour picked for its own sake. */
export type ReadingTone = "completed-success" | "awaiting-review" | "completed-failed" | "notice-caution";

/** A reading, in `FleetPanel`'s own terms. */
export type MachineReading = {
  label: string;
  value?: ReactNode;
  valueFace?: "sans" | "mono";
  tone?: ReadingTone;
  detail?: ReactNode;
  detailFace?: "sans" | "mono";
};

/** The status bar's three hues, and nothing for the states that are none of them. */
const FLEET_TONE: Partial<Record<Connection["state"], ReadingTone>> = {
  connected: "completed-success",
  not_running: "completed-failed",
  unreachable: "awaiting-review",
};

/** The connection's own statement — the status bar's sentence, not a second one. */
export function fleetReading(connection: Connection, now: number, readAt: number | null): MachineReading {
  const statement = statementOf(connection, now, readAt);
  const tone = FLEET_TONE[connection.state];
  return {
    label: "Fleet",
    value: statement.headline,
    ...(tone === undefined ? {} : { tone }),
    ...(statement.detail === "" ? {} : { detail: statement.detail, detailFace: "mono" as const }),
  };
}

/** Doctor's words, worst first. A word this build does not know ranks below all three. */
const OUTCOMES = ["fail", "warn", "pass"] as const;
const DOCTOR_TONE: Record<(typeof OUTCOMES)[number], ReadingTone> = {
  fail: "completed-failed",
  warn: "awaiting-review",
  pass: "completed-success",
};

export function doctorReading(read: HealthRead): MachineReading {
  switch (read.state) {
    case "none":
    case "reading":
      return { label: "Doctor" };
    case "failed":
      return { label: "Doctor", value: "Not read", detail: said(read.outcome) };
    case "read":
      return doctorOf(read.health);
  }
}

/**
 * The worst word, and every module that did not pass named beside its own. **No blended score**:
 * a failing module is named rather than folded into a count a passing one could hide it in.
 */
function doctorOf(health: FleetHealth): MachineReading {
  const first = health.probes[0];
  if (first === undefined) return { label: "Doctor", value: "Nothing probed" };
  const worst = OUTCOMES.find((word) => health.probes.some((probe) => probe.outcome === word));
  const flagged = health.probes.filter((probe) => probe.outcome !== "pass");
  const detail =
    flagged.length > 0
      ? flagged.map((probe) => `${probe.module}: ${probe.outcome}`).join(" · ")
      : health.probes.map((probe) => probe.module).join(", ");
  return {
    label: "Doctor",
    value: worst ?? first.outcome,
    valueFace: "mono",
    ...(worst === undefined ? {} : { tone: DOCTOR_TONE[worst] }),
    detail,
  };
}

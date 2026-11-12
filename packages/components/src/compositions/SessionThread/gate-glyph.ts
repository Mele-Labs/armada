import type { PlanGate } from "@armada/protocol";
import {
  CircleCheck,
  Footprints,
  GitMerge,
  Link,
  NotebookPen,
  PencilRuler,
  ShieldCheck,
  Stamp,
  type LucideIcon,
} from "lucide-react";

/** A gate that ends something: every `PlanGate` but `none`. */
export type GateKind = Exclude<PlanGate, "none">;

/** The glyph of each gate, on a plan's Now and on a proposed workflow's lanes alike. */
export const GATE_GLYPH: Record<GateKind, LucideIcon> = {
  plan_approved: Stamp,
  walk_approved: Footprints,
  sketch_approved: PencilRuler,
  document_approved: NotebookPen,
  you_approved: CircleCheck,
  checks_green: ShieldCheck,
  merged: GitMerge,
  // The Jobs tab's glyph: the Jobs this one started.
  jobs_ended: Link,
};

/** The gate's name, for its glyph's tooltip. */
export const GATE_NAMED: Record<GateKind, string> = {
  plan_approved: "Your approval of the plan",
  walk_approved: "Your approval of the walk",
  sketch_approved: "Your approval of the sketch",
  document_approved: "Your approval of the document",
  you_approved: "Your Approve",
  checks_green: "Checks pass",
  merged: "The merge",
  jobs_ended: "The Jobs it started end",
};

// The dispatch kinds' name and mark, shared by the panel's line and the strip left after a send.

import { Plane, Send, SquareTerminal, type LucideIcon } from "lucide-react";
import type { DispatchKind } from "@armada/components";

/** The kinds, by the word that names them. Tab steps through the ones offered. */
export const KIND_LABEL: Record<DispatchKind, string> = { job: "Job", session: "Session", autopilot: "Autopilot" };

/** Each kind's mark on the line: the registry's Dispatch, Session and Autopilot glyphs. */
export const KIND_ICON: Record<DispatchKind, LucideIcon> = { job: Send, session: SquareTerminal, autopilot: Plane };

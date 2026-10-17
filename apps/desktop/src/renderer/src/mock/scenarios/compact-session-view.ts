// The compact Session view with its calls pinned over the thread, as they are. The walk `compactSessionView` plays it. Built in `../compact-session.ts`.

import { compactSession } from "../compact-session";
import type { Scenario } from "../moment";

export const s271CompactSessionView: Scenario = compactSession("compact-session-view", "A Session open with less on screen: Progress and Flight control's keys for Artifacts, Subagents, Checks and Slot, calls pinned");

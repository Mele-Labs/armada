// The compact Session with a question waiting, answered in the composer: one line over the input and its options as numbered keys. The walk `compactCallsComposer`.

import { compactSession } from "../compact-session";
import type { Scenario } from "../moment";

export const s272CompactCallsComposer: Scenario = compactSession("compact-calls-composer", "A Session with a question waiting, answered in the composer's line", "composer");

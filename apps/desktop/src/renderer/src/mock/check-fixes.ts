// Mock-only: the three Check fixes main has no route for (`retry_check`, `skip_check`, `skip_checks`
// are on Job 14's branch). Each answers after a beat and reaches nothing, so a card shows its pending
// state and leaves. `fake.ts` registers them beside its api; the real preload registers none.

import type { CheckFixes } from "../cockpit/fix-job";

const BEAT_MS = 350;
const taken = () => new Promise<{ ok: true }>((resolve) => setTimeout(() => resolve({ ok: true }), BEAT_MS));

export const mockCheckFixes: CheckFixes = { retryCheck: taken, skipCheck: taken, skipChecks: taken };

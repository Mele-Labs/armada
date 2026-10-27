// What he picked in an empty Mod Job, kept by the Job like a draft is: it survives leaving and coming back, and goes with his first message.

import { forgetKept, readKept } from "@armada/components";
import type { ModAim } from "@armada/protocol";

const AIMS: readonly ModAim[] = ["theme", "layout", "words", "fleet", "tracker", "commands", "source"];
export const aimKey = (sessionId: string) => `mod-aim:${sessionId}`;

export const aimOf = (sessionId: string): ModAim | undefined => {
  const kept = readKept<unknown>(aimKey(sessionId));
  return AIMS.find((one) => one === kept);
};

export const forgetAim = (sessionId: string): void => forgetKept(aimKey(sessionId));

import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import { Laptop, Unplug } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { SessionState } from "../SessionFrame/SessionFrame";

/**
 * Another machine's Sessions, signed in under the same GitHub login. **Presence
 * only**: each Fleet publishes a trimmed summary when a Session changes and reads
 * the other's. A summary has the id, the repository's name, the title, the
 * state, since when and what it waits on, and never message text, code or a
 * transcript, so a row built from one has nothing to open and nothing to act on.
 * Nothing is drawn where no machine is provided.
 */
export type OtherSession = {
  id: string;
  repository: string;
  title: string;
  state: Extract<SessionState, "working" | "waiting" | "idle" | "ended">;
  /** When it reached this state. */
  since: string;
  /** What it waits on the person for, in the words it published. */
  waiting?: readonly string[];
};

export type OtherMachine = {
  /** The name the machine goes by. */
  name: string;
  sessions: readonly OtherSession[];
  /** Set once the machine has stopped being heard from: when it last was. */
  lastHeard?: string;
  /** How the Cockpit's Your move shows this machine's waiting items, while the owner chooses between the two. Mock only. */
  onCockpit?: "strip" | "cards";
};

const Held = createContext<OtherMachine | undefined>(undefined);

export function OtherMachineProvider({ value, children }: { value: OtherMachine; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

/** The other machine this window reads. `undefined` where there is none. */
export function useOtherMachine(): OtherMachine | undefined {
  return useContext(Held);
}

/** Marks a row as another machine's, and names the machine on hover. */
export function MachineMark({ name }: { name: string }) {
  return (
    <Tooltip label={name}>
      <span className="armada-session-mark" role="img" aria-label={name}>
        <Laptop size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** A machine that has gone quiet: how long since it was last heard, on hover. */
export function QuietMark({ said }: { said: string }) {
  return (
    <Tooltip label={said}>
      <span className="armada-session-mark" role="img" aria-label={said}>
        <Unplug size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

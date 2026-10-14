// The other machine under this login, as the mock holds it: a scenario's summaries, changed one moment at
// a time as a walk lets time pass. Fleet serves none of it yet. Nothing carried, nothing is drawn.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { OtherMachineProvider } from "@armada/components";
import type { OtherMachine } from "@armada/components";

import type { MockDraft } from "./moment";
import { alsoOnTimePassing } from "./time-passes";

function Held({ other, children }: { other: NonNullable<MockDraft["otherMachine"]>; children: ReactNode }) {
  const [machine, setMachine] = useState<OtherMachine>(other.machine);
  const moment = useRef(0);
  useEffect(() => {
    alsoOnTimePassing(() => {
      const next = other.moments[moment.current];
      if (next === undefined) return;
      moment.current += 1;
      setMachine(next);
    });
    return () => alsoOnTimePassing(undefined);
  }, [other]);
  return <OtherMachineProvider value={machine}>{children}</OtherMachineProvider>;
}

export function OtherMachineFrom({ other, children }: { other: MockDraft["otherMachine"]; children: ReactNode }) {
  return other === undefined ? <>{children}</> : <Held other={other}>{children}</Held>;
}

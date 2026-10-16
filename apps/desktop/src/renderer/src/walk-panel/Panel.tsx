// The panel's content, apart from the entry that mounts it, so a test draws it on the mock Fleet.

import { useEffect, useState } from "react";

import { NOTHING_YET, type BridgeState } from "../../../shared/bridge";
import { stateOf } from "../state-feed";
import type { WalkSubject } from "./subject";
import { WalkJob } from "./WalkJob";
import { WalkSession } from "./WalkSession";

/** The Session or the Job this window walks, over the state main publishes to every Bridge document. */
export function Panel({ subject }: { subject: WalkSubject | null }) {
  const [state, setState] = useState<BridgeState>(NOTHING_YET);
  useEffect(() => {
    let live = true;
    const source = stateOf(window.armada);
    void source.state().then((read) => live && read !== undefined && setState(read));
    const off = source.subscribe(setState);
    return () => {
      live = false;
      off();
    };
  }, []);
  if (subject === null) return null;
  return subject.kind === "session" ? (
    <WalkSession sessionId={subject.id} />
  ) : (
    <WalkJob jobId={subject.id} state={state} onRead={setState} />
  );
}

// The words a Job's call card sends to its Drone, in the Session's own box with nothing but its field and
// its Send. A Job has no Session to tune, so the mode, model, effort and attachments are not drawn.

import { SessionComposer } from "@armada/components";

const nothing = (): void => undefined;

/** `draftKey` keeps what was typed across leaving the card; `onSend` takes the words. */
export function JobBox({ draftKey, onSend }: { draftKey: string; onSend: (words: string) => void }) {
  return (
    <SessionComposer
      bare
      working={false}
      mode="auto"
      onMode={nothing}
      model={null}
      effort={null}
      models={[]}
      efforts={[]}
      onTune={nothing}
      commands={[]}
      taggable={[]}
      tags={[]}
      onTags={nothing}
      compact
      drawn={[]}
      onDraw={nothing}
      onRemoveDrawn={nothing}
      draftKey={draftKey}
      onSend={(sent) => onSend(sent.text)}
    />
  );
}

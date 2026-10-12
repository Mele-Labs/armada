// What Fleet read of an Autopilot sentence, in the dispatch panel's body: the instruction and the four
// settings, each editable in mono, as the Cockpit lane's start form lays them out. Nothing starts
// until the panel's act is pressed. The fields, and the words each was read from, are the lane's own.

import { AutopilotFieldInputs, Input, type AutopilotReading } from "@armada/components";

/** The reading as the panel holds it while the person checks it: Fleet's instruction, and the fields. */
export type AutopilotRead = AutopilotReading & { instruction: string };

export function AutopilotBody({
  read,
  onRead,
  refused,
}: {
  /** Absent while the sentence has not been read, or a refusal is all there is to say. */
  read: AutopilotRead | undefined;
  onRead: (read: AutopilotRead) => void;
  /** What Fleet or the fields refused, in words. */
  refused: string | undefined;
}) {
  return (
    <div className="armada-dispatch-panel__autopilot">
      {read === undefined ? null : (
        <>
          <Input label="Instruction" mono value={read.instruction} onChange={(event) => onRead({ ...read, instruction: event.currentTarget.value })} />
          <AutopilotFieldInputs fields={read.fields} onChange={(fields) => onRead({ ...read, fields })} quoted={read.quoted} />
        </>
      )}
      {refused === undefined ? null : (
        <p className="armada-autopilot__refused" role="alert">
          {refused}
        </p>
      )}
    </div>
  );
}

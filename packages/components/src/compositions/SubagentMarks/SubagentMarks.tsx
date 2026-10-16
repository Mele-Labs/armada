import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { modelSaid } from "../SessionFrame/model-said";

/**
 * The type and the model a subagent runs on, as bare words under its sheet's title. Each is left out
 * until it is known, and nothing stands in for it. The context figure beside them is the Session's own
 * `ContextFigure`.
 */
export function SubagentMarks({ type, model }: { type?: string | undefined; model?: string | undefined }) {
  if (type === undefined && model === undefined) return null;
  return (
    <span className="armada-subagent-marks">
      {type === undefined ? null : (
        <Tooltip label="Agent type">
          <span className="armada-subagent-marks__type">{type}</span>
        </Tooltip>
      )}
      {model === undefined ? null : (
        <Tooltip label="Model">
          <span>{modelSaid(model)}</span>
        </Tooltip>
      )}
    </span>
  );
}

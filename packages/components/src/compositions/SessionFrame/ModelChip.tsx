import { Cpu } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** What Claude Code runs a Session on when none was picked, which Fleet does not say. */
const DEFAULT_MODEL = "Opus 5.5";

/** The name a model is shown by. None picked is the default, named and not left blank. */
export function modelSaid(model: string | undefined): string {
  if (model === undefined || model === "" || model === "opus") return DEFAULT_MODEL;
  return model.charAt(0).toUpperCase() + model.slice(1);
}

/** The model a Session runs on, always named: a picked one, or the default where none was. */
export function ModelChip({ model }: { model: string | undefined }) {
  const name = modelSaid(model);
  return (
    <Tooltip label={model === undefined || model === "" ? "Model · default" : "Model"}>
      <span className="armada-session-frame__model" role="img" aria-label={`Model ${name}`}>
        <Cpu size={12} strokeWidth={2} aria-hidden />
        {name}
      </span>
    </Tooltip>
  );
}

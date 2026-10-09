// Mods on this machine: one row each, with whether it is on, what is wrong with it if anything, and Promote.

import { GitPullRequestArrow } from "lucide-react";
import { Button, Card, CardHeader, Switch, Tooltip } from "@armada/components";

import { useThemes } from "./theme-source";

export function ModsSurface() {
  const { mods, source } = useThemes();
  return (
    <div className="armada-screen__pane">
      {mods.map((mod) => (
        <Card key={mod.name}>
          <CardHeader role="group" aria-label={mod.title}>
            <Switch checked={mod.enabled} onChange={(event) => source.setEnabled(mod.name, event.target.checked)} description={mod.problem ?? mod.branch}>
              {mod.title}
            </Switch>
            <Tooltip label={`Put ${mod.title} on a branch of the repository`}>
              <Button iconOnly variant="ghost" aria-label={`Promote ${mod.title}`} disabled={mod.problem !== undefined || mod.branch !== undefined} onClick={() => source.promote(mod.name)}>
                <GitPullRequestArrow size={16} />
              </Button>
            </Tooltip>
          </CardHeader>
        </Card>
      ))}
    </div>
  );
}

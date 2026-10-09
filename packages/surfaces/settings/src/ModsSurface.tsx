// Mods on this machine: one row each, with the kind, whether it is on, and Promote. Mock data.

import { Palette, GitPullRequestArrow } from "lucide-react";
import { Button, Card, CardContent, Switch, Tooltip } from "@armada/components";

import { enableMod, promoteMod, useMods } from "./mods";

export function ModsSurface() {
  const { mods } = useMods();
  return (
    <div className="armada-screen__pane">
      {mods.map((mod) => (
        <Card key={mod.name}>
          <CardContent>
            <div role="group" aria-label={mod.title} style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <Tooltip label="Theme">
                <span role="img" aria-label="Theme" style={{ display: "inline-flex", color: "var(--fg-muted)" }}>
                  <Palette size={16} />
                </span>
              </Tooltip>
              <div style={{ flex: 1 }}>
                <Switch checked={mod.enabled} onChange={(event) => enableMod(mod.name, event.target.checked)}>
                  {mod.title}
                </Switch>
                {mod.promoted && <div style={{ color: "var(--fg-muted)", fontSize: 12 }}>Branch mods/{mod.name} pushed. Pull request open.</div>}
              </div>
              <Tooltip label={`Promote ${mod.title} to a branch and pull request`}>
                <Button iconOnly variant="ghost" aria-label={`Promote ${mod.title}`} disabled={mod.promoted} onClick={() => promoteMod(mod.name)}>
                  <GitPullRequestArrow size={16} />
                </Button>
              </Tooltip>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

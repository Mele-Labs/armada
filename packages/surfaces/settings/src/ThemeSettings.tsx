// Settings → Theme: Dark, Light, and a theme for each enabled mod that ships one.

import { Puzzle } from "lucide-react";
import { Radio, RadioGroup, Tooltip } from "@armada/components";

import { useThemes } from "./theme-source";

export function ThemeSettings() {
  const { mods, active, source } = useThemes();
  const choices = [{ name: "dark", title: "Dark", mod: false }, { name: "light", title: "Light", mod: false }, ...mods.filter((one) => one.enabled).map((one) => ({ name: one.name, title: one.title, mod: true }))];
  return (
    <RadioGroup label="Theme">
      {choices.map((one) => (
        <div key={one.name} style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Radio name="theme" checked={active === one.name} onChange={() => source.setActive(one.name)}>
            {one.title}
          </Radio>
          {one.mod && (
            <Tooltip label={`From the ${one.name} mod`}>
              <span role="img" aria-label={`From the ${one.name} mod`} style={{ display: "inline-flex", color: "var(--fg-subtle)" }}>
                <Puzzle size={14} />
              </span>
            </Tooltip>
          )}
        </div>
      ))}
    </RadioGroup>
  );
}

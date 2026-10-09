// Armada Mods, tier 1: themes. Mock only, drawn so the owner can look before anything is wired. The
// Theme picker, the Mods list and a Session that writes a theme are one window; switching a theme
// overrides custom properties on the window's root. `walks/mods-themes.ts` plays it.

import { useState } from "react";
import { FilePlus, GitPullRequestArrow, Palette, Puzzle, SendHorizontal } from "lucide-react";
import { Button, Switch, Textarea, Tooltip } from "@armada/components";

import { MODS_DIR, THEMES } from "./themes";
import type { ThemeId } from "./themes";

type Mod = { id: ThemeId; enabled: boolean; promoted: boolean };
type Turn = { who: "owner" | "session"; text: string; wrote?: string[] };
type Tab = "Theme" | "Mods" | "Session";

const REQUEST = "Make a dusk theme: purple-grey ground, a warm accent.";

export function ModsMock() {
  const [tab, setTab] = useState<Tab>("Theme");
  const [theme, setTheme] = useState<ThemeId>("dark");
  const [mods, setMods] = useState<Mod[]>([
    { id: "nord", enabled: true, promoted: false },
    { id: "solarized", enabled: true, promoted: false },
  ]);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [draft, setDraft] = useState(REQUEST);

  const live = mods.filter((one) => one.enabled).map((one) => one.id);
  const choices: ThemeId[] = ["dark", "light", ...live];
  const shown = choices.includes(theme) ? theme : "dark";

  const send = () => {
    if (draft.trim() === "") return;
    setTurns([
      { who: "owner", text: draft },
      { who: "session", text: "Dusk is in Theme.", wrote: [`${MODS_DIR}/dusk/mod.toml`, `${MODS_DIR}/dusk/theme.css`] },
    ]);
    setMods((all) => (all.some((one) => one.id === "dusk") ? all : [...all, { id: "dusk", enabled: true, promoted: false }]));
    setDraft("");
  };
  const flip = (id: ThemeId, enabled: boolean) => setMods((all) => all.map((one) => (one.id === id ? { ...one, enabled } : one)));
  const promote = (id: ThemeId) => setMods((all) => all.map((one) => (one.id === id ? { ...one, promoted: true } : one)));

  return (
    <div
      data-theme={shown}
      style={{ ...THEMES[shown].vars, background: "var(--bg-base)", color: "var(--fg-default)", minHeight: "100vh", padding: 24, display: "grid", gap: 16, alignContent: "start" }}
    >
      <nav role="tablist" aria-label="Settings" style={{ display: "flex", gap: 8 }}>
        {(["Theme", "Mods", "Session"] as const).map((name) => (
          <Button key={name} role="tab" aria-selected={tab === name} variant={tab === name ? "tonal" : "ghost"} onClick={() => setTab(name)}>
            {name}
          </Button>
        ))}
      </nav>

      {tab === "Theme" && (
        <section aria-label="Theme" style={{ display: "grid", gap: 8, maxWidth: 420 }}>
          {choices.map((id) => (
            <div key={id} style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Button variant={shown === id ? "primary" : "secondary"} aria-pressed={shown === id} onClick={() => setTheme(id)} style={{ flex: 1, justifyContent: "flex-start" }}>
                {THEMES[id].name}
              </Button>
              {id !== "dark" && id !== "light" && (
                <Tooltip label={`From the ${id} mod`}>
                  <span aria-label={`From the ${id} mod`} style={{ color: "var(--fg-subtle)", display: "inline-flex" }}>
                    <Puzzle size={16} />
                  </span>
                </Tooltip>
              )}
            </div>
          ))}
        </section>
      )}

      {tab === "Mods" && (
        <section aria-label="Mods" style={{ display: "grid", gap: 8, maxWidth: 560 }}>
          {mods.map((mod) => (
            <div key={mod.id} role="group" aria-label={mod.id} style={{ display: "flex", alignItems: "center", gap: 12, padding: 12, background: "var(--bg-raised)", border: "1px solid var(--border-default)", borderRadius: 8 }}>
              <Tooltip label="Theme">
                <span aria-label="Theme" style={{ display: "inline-flex", color: "var(--fg-muted)" }}>
                  <Palette size={16} />
                </span>
              </Tooltip>
              <div style={{ flex: 1 }}>
                <Switch checked={mod.enabled} onChange={(event) => flip(mod.id, event.target.checked)}>
                  {mod.id}
                </Switch>
                {mod.promoted && (
                  <div style={{ color: "var(--fg-muted)", fontFamily: "var(--font-mono, monospace)", fontSize: 12, marginTop: 4 }}>
                    Branch mods/{mod.id} pushed. Pull request open.
                  </div>
                )}
              </div>
              <Tooltip label={`Promote ${mod.id} to a branch and pull request`}>
                <Button iconOnly variant="ghost" aria-label={`Promote ${mod.id}`} disabled={mod.promoted} onClick={() => promote(mod.id)}>
                  <GitPullRequestArrow size={16} />
                </Button>
              </Tooltip>
            </div>
          ))}
        </section>
      )}

      {tab === "Session" && (
        <section aria-label="Session" style={{ display: "grid", gap: 12, maxWidth: 560 }}>
          {turns.map((turn) => (
            <div key={turn.who} style={{ padding: 12, background: turn.who === "owner" ? "var(--accent-muted)" : "var(--bg-raised)", border: "1px solid var(--border-default)", borderRadius: 8, display: "grid", gap: 8 }}>
              <span>{turn.text}</span>
              {turn.wrote?.map((path) => (
                <Tooltip key={path} label="Written">
                  <code style={{ display: "flex", gap: 8, alignItems: "center", background: "var(--bg-sunken)", padding: "4px 8px", borderRadius: 4, fontSize: 12, color: "var(--fg-muted)" }}>
                    <FilePlus size={14} />
                    {path}
                  </code>
                </Tooltip>
              ))}
            </div>
          ))}
          <div style={{ display: "flex", gap: 8, alignItems: "flex-end" }}>
            <div style={{ flex: 1 }}>
              <Textarea aria-label="Message" value={draft} onChange={(event) => setDraft(event.target.value)} />
            </div>
            <Tooltip label="Send">
              <Button iconOnly variant="primary" aria-label="Send" onClick={send}>
                <SendHorizontal size={16} />
              </Button>
            </Tooltip>
          </div>
        </section>
      )}
    </div>
  );
}

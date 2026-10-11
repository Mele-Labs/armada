// Every setting on this machine, on one screen. #1089 — split into groups with a search over every
// setting, one group drawn at a time with each of its sections under its own heading (`./sections.ts`).
//
// **settings.json is the source.** Fleet's schema decides the categories and the controls
// (`./SchemaSettings.tsx`), so a key Fleet adds is drawn without a change here; Phone, Theme,
// Layout, Keyboard shortcuts and Guides keep their own panes.
//
// **`BridgeSettings`, not `Settings`.** `./settings.tsx` is a Job's own — `SettingsSheet`, its caps
// and its choices — and the two would collide by name and, on a case-insensitive filesystem, by file.

import { Card, CardContent, CardHeader, CardTitle, GuidesSetting, SettingsIndex } from "@armada/components";
import type { Outcome, SaveSettings, SettingsList } from "@armada/protocol";
import { useEffect, useState, type ReactNode } from "react";

import type { SettingsFileOpened } from "./api";
import { KeyboardSettings, keyboardMatches } from "./KeyboardSettings";
import { LayoutSettings } from "./LayoutSettings";
import { PhoneSettings } from "./PhoneSettings";
import { SchemaSection, SettingsFile } from "./SchemaSettings";
import { ThemeSettings } from "./ThemeSettings";
import {
  groupsFor,
  matches,
  matchesIn,
  settingsIn,
  takeSettingsFocus,
  useSettingsAsked,
  useSettingsSection,
  wordsOf,
  type SettingsSection,
} from "./sections";

/** The element a section is drawn in, which the palette scrolls to. */
const anchorOf = (section: string) => `settings-${section}`;

export type BridgeSettingsProps = {
  /** settings.json as Fleet resolved it. `null` where Fleet has not answered — the schema's categories say so. */
  settings: SettingsList | null;
  /** A live connection. Off with nothing to send to. */
  live: boolean;
  /** Change settings by key; `null` removes one, so the shipped value is back. */
  onSaveSettings: (changes: SaveSettings["changes"]) => Promise<Outcome>;
  /** Open settings.json in the configured editor. */
  onOpenSettingsFile: () => Promise<SettingsFileOpened>;
  /** Opens the guide catalogue. Navigation is the window's, so it arrives as a prop. */
  onReadGuides?: () => void;
  /** A clipboard write is silent, so the window confirms it. */
  onCopied?: (value: string) => void;
};

export function BridgeSettings({ settings, live, onSaveSettings, onOpenSettingsFile, onReadGuides, onCopied }: BridgeSettingsProps) {
  const [chosen, setCurrent] = useSettingsSection();
  const asked = useSettingsAsked();
  const [query, setQuery] = useState("");
  const searching = query.trim() !== "";
  const groups = groupsFor(settings);
  // A group this reading does not have draws the first.
  const current = groups.some((one) => one.id === chosen) ? chosen : (groups[0]?.id ?? chosen);
  const found = (section: SettingsSection) =>
    matchesIn(section, query, settings) + (section.id === "keyboard" ? keyboardMatches(query) : 0);
  const counted = groups.map((group) => ({
    id: group.id,
    label: group.label,
    matches: searching ? group.sections.reduce((sum, section) => sum + found(section), 0) : 0,
  }));
  // While a search is on, every group with a match, and in each only the sections holding one.
  const shown = searching
    ? groups.flatMap((group) => group.sections.filter((section) => found(section) > 0).map((section) => ({ group, section })))
    : (groups.find((one) => one.id === current)?.sections ?? []).map((section) => ({ group: undefined, section }));

  // The palette opened a section: its group is drawn, so scroll it into view.
  useEffect(() => {
    const wanted = takeSettingsFocus();
    if (wanted !== null) document.getElementById(anchorOf(wanted))?.scrollIntoView({ block: "start" });
  }, [asked, current]);

  const card = (title: string, body: ReactNode) => (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>
      <CardContent>{body}</CardContent>
    </Card>
  );

  const panes: Record<string, (title: string) => ReactNode> = {
    phone: () => <PhoneSettings {...(onCopied === undefined ? {} : { onCopied })} />,
    theme: (title) => card(title, <ThemeSettings />),
    layout: (title) => card(title, <LayoutSettings />),
    keyboard: (title) => card(title, <KeyboardSettings query={query} />),
    guides: (title) => card(title, <GuidesSetting {...(onReadGuides === undefined ? {} : { onReadGuides })} />),
  };

  /** One section under its heading: `Group › Section` while searching, since matches from several groups share the page. */
  const body = (section: SettingsSection, group: string | undefined): ReactNode => {
    const title = group === undefined || group === section.label ? section.label : `${group} › ${section.label}`;
    const own = panes[section.id];
    if (section.schema !== true && own !== undefined) return own(title);
    const all = settings === null ? null : settingsIn(settings, section.id);
    const hits = all === null || !searching ? all : all.filter((one) => matches(wordsOf(one), query));
    return <SchemaSection label={title} settings={hits} live={live} onSave={onSaveSettings} />;
  };

  return (
    <div className="armada-screen__pane">
      <SettingsIndex sections={counted} current={current} onSection={setCurrent} query={query} onQuery={setQuery}>
        <SettingsFile settings={settings} onOpen={onOpenSettingsFile} />
        {shown.map(({ group, section }) => (
          <div key={section.id} id={anchorOf(section.id)}>
            {body(section, group?.label)}
          </div>
        ))}
      </SettingsIndex>
    </div>
  );
}

// A few of settings.json's categories drawn on their own, one card each, for a page that places them
// itself. The same bodies `BridgeSettings` draws; this is the way to ask for some by id.

import { Button, Card, CardContent, CardHeader, CardTitle, GuidesSetting } from "@armada/components";
import type { Outcome, SaveSettings, SettingsList } from "@armada/protocol";
import type { ComponentProps, ReactNode } from "react";

import { KeyboardSettings } from "./KeyboardSettings";
import { LayoutSettings } from "./LayoutSettings";
import { ModList } from "./ModList";
import { PhoneSettings } from "./PhoneSettings";
import { ReachFoldersSettings } from "./ReachFoldersSettings";
import { SchemaSection } from "./SchemaSettings";
import { SETTINGS_SECTIONS, settingsIn } from "./sections";
import { ThemeSettings } from "./ThemeSettings";
import { TimeSettings } from "./TimeSettings";

export type SettingsSectionsViewProps = {
  /** The categories to draw, in this order, by id (`sections.ts`). */
  ids: readonly string[];
  settings: SettingsList | null;
  live: boolean;
  onSaveSettings: (changes: SaveSettings["changes"]) => Promise<Outcome>;
  /** Opens the account page or panel: a link from the Account category. */
  onOpenAccount?: () => void;
  /** Opens the guide catalogue, from the guides setting. */
  onReadGuides?: () => void;
  onCopied?: (value: string) => void;
  /** Drawn directly under a category, by id. */
  under?: Readonly<Record<string, ReactNode>>;
  /** Open a promoted mod's pull request and its Job from the All list, as the Theme and Layout pages do. */
  onOpenPull?: ComponentProps<typeof ModList>["onOpenPull"];
  onOpenJob?: ComponentProps<typeof ModList>["onOpenJob"];
};

export function SettingsSectionsView({ ids, settings, live, onSaveSettings, onOpenAccount, onReadGuides, onCopied, under = {}, onOpenPull, onOpenJob }: SettingsSectionsViewProps) {
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
    "mods-all": (title) => <ModList kind="all" title={title} {...(onOpenPull === undefined ? {} : { onOpenPull })} {...(onOpenJob === undefined ? {} : { onOpenJob })} />,
    theme: (title) => card(title, <ThemeSettings />),
    layout: (title) => card(title, <LayoutSettings />),
    words: (title) => <ModList kind="words" title={title} />,
    trackers: (title) => <ModList kind="tracker" title={title} />,
    "fleet-mods": (title) => <ModList kind="fleet" title={title} />,
    time: (title) => card(title, <TimeSettings />),
    keyboard: (title) => card(title, <KeyboardSettings query="" />),
    guides: (title) => card(title, <GuidesSetting {...(onReadGuides === undefined ? {} : { onReadGuides })} />),
  };

  const body = (id: string): ReactNode => {
    const section = SETTINGS_SECTIONS.find((one) => one.id === id);
    if (section === undefined) return null;
    const own = panes[id];
    if (section.schema !== true && own !== undefined) return own(section.label);
    const schema = <SchemaSection label={section.label} settings={settings === null ? null : settingsIn(settings, id)} live={live} onSave={onSaveSettings} />;
    // The folder list is JSON, which the schema's form does not draw.
    if (id === "features") {
      return (
        <>
          {schema}
          <ReachFoldersSettings settings={settings} live={live} onSave={onSaveSettings} />
        </>
      );
    }
    if (id !== "account" || onOpenAccount === undefined) return schema;
    return (
      <>
        <div className="armada-settings__link">
          <Button variant="secondary" size="sm" ground="card" onClick={onOpenAccount}>
            Account
          </Button>
        </div>
        {schema}
      </>
    );
  };

  return (
    <>
      {ids.map((id) => (
        <div key={id} id={`settings-${id}`} className="armada-system__section">
          {body(id)}
          {under[id]}
        </div>
      ))}
    </>
  );
}

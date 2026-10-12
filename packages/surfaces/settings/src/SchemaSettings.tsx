// settings.json, drawn from Fleet's schema: one card per section, one `SettingField` per setting,
// and the file itself above them — where it is, the way to open it, and why Fleet refused it.
//
// **The control is the kind's, never the key's.** Nothing here names a setting, so a key Fleet adds
// is drawn the day it ships; the one Bridge reads itself, `features.openGuidesFirstTime`, is wired
// in `App.tsx`, not here.

import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  inWords,
  SettingField,
  SettingFieldList,
  SettingsFileBar,
  type SettingFieldControl,
  type SettingFieldValue,
} from "@armada/components";
import type { Outcome, SaveSettings, Setting, SettingsList, SettingValue } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { useState } from "react";

import type { SettingsFileOpened } from "./api";

/** Fleet-wide, so the line every section's card opens on says when a change counts. */
const LEAD = "These live in settings.json. A change counts at once, unless a setting says Fleet has to restart first.";

/** Why nothing can be sent. */
const NOT_LIVE = "Fleet is not connected, so nothing can be changed.";

/** A setting's figure as the field and its shipped line read it. */
function asNumber(value: SettingValue, fallback: number): number {
  return typeof value === "number" ? value : fallback;
}

/** `2`, `15%`, `10 GiB`, `$10`. */
function figure(value: number, unit: string | null | undefined): string {
  if (unit === undefined || unit === null || unit === "") return String(value);
  if (unit === "%") return `${value}%`;
  if (unit === "$") return `$${value}`;
  return `${value} ${unit}`;
}

/**
 * The setting a refused save is drawn against: the one `fleet.unacceptable_settings` names in its
 * `key` field where this section holds it, or else the one the save sent.
 */
export function refusedAt(outcome: Outcome, sent: string, held: readonly Setting[] | null): string {
  const named = !outcome.ok && outcome.why === "refused" ? outcome.error.fields["key"] : undefined;
  return typeof named === "string" && (held ?? []).some((one) => one.key === named) ? named : sent;
}

/** The control a setting's kind draws, with the value in force; `null` for one the form does not draw. */
function controlOf(setting: Setting): { control: SettingFieldControl; shipped?: string } | null {
  const { kind } = setting;
  switch (kind.kind) {
    case "integer": {
      const shipped = asNumber(setting.default, kind.min);
      return {
        control: { kind: "number", value: asNumber(setting.value, shipped), min: kind.min, max: kind.max, ...(kind.unit ? { unit: kind.unit } : {}) },
        shipped: figure(shipped, kind.unit),
      };
    }
    case "seconds": {
      const shipped = asNumber(setting.default, kind.min);
      return { control: { kind: "number", value: asNumber(setting.value, shipped), min: kind.min, max: kind.max, seconds: true }, shipped: inWords(shipped) };
    }
    case "boolean":
      return { control: { kind: "switch", value: setting.value === true }, shipped: setting.default === true ? "on" : "off" };
    case "choice":
      return { control: { kind: "choice", value: String(setting.value), options: kind.options } };
    case "text":
      return { control: { kind: "text", value: typeof setting.value === "string" ? setting.value : "" } };
    case "prompt":
      return { control: { kind: "prompt", value: typeof setting.value === "string" ? setting.value : "" } };
    case "text_list":
      return { control: { kind: "list", value: Array.isArray(setting.value) ? setting.value.map(String) : [] } };
    case "json":
      return null;
  }
}

export type SchemaSectionProps = {
  label: string;
  /** The section's settings, or only those a search found. `null` before Fleet has answered. */
  settings: readonly Setting[] | null;
  live: boolean;
  onSave: (changes: SaveSettings["changes"]) => Promise<Outcome>;
};

/**
 * One section of settings.json. **One save in flight for the whole section**:
 * main sends one at a time, and a second press here could only be refused.
 */
export function SchemaSection({ label, settings, live, onSave }: SchemaSectionProps) {
  const [saving, setSaving] = useState<string | null>(null);
  const [refused, setRefused] = useState<Record<string, string>>({});

  function send(key: string, value: SettingFieldValue | null): void {
    setSaving(key);
    setRefused(({ [key]: _gone, ...rest }) => rest);
    const sent: SettingValue | null = value === null ? null : Array.isArray(value) ? [...value] : (value as SettingValue);
    void onSave({ [key]: sent }).then((outcome) => {
      setSaving(null);
      if (!outcome.ok) setRefused((was) => ({ ...was, [refusedAt(outcome, key, settings)]: refusalWords(outcome) }));
    });
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{label}</CardTitle>
      </CardHeader>
      <CardContent>
        {settings === null ? (
          <p className="armada-settings__quiet">Fleet has not answered yet, so these settings are not here.</p>
        ) : (
          <SettingFieldList lead={LEAD} {...(live ? {} : { note: NOT_LIVE })}>
            {settings.map((setting) => {
              const drawn = controlOf(setting);
              if (drawn === null) return null;
              const said = refused[setting.key];
              return (
                <SettingField
                  key={setting.key}
                  name={setting.key}
                  title={setting.title}
                  description={setting.description}
                  control={drawn.control}
                  {...(drawn.shipped === undefined ? {} : { shipped: drawn.shipped })}
                  modified={(setting.saved ?? null) !== null}
                  restart={setting.pending_restart}
                  {...(setting.overridden_by_env == null ? {} : { overriddenBy: setting.overridden_by_env })}
                  disabled={!live || (saving !== null && saving !== setting.key)}
                  saving={saving === setting.key}
                  {...(said === undefined ? {} : { refused: said })}
                  onSave={(value) => send(setting.key, value)}
                  onReset={() => send(setting.key, null)}
                />
              );
            })}
          </SettingFieldList>
        )}
      </CardContent>
    </Card>
  );
}

export type SettingsFileProps = {
  settings: SettingsList | null;
  onOpen: () => Promise<SettingsFileOpened>;
};

/** Why the file did not open, in the row's voice, or `null` because it did. */
function whyNotOpened(opened: SettingsFileOpened): string | null {
  if (opened.ok) return null;
  if (opened.why === "not_read") return "Fleet has not said where settings.json is yet. It does once it answers.";
  return `This machine did not open it: ${opened.detail}`;
}

/** settings.json itself, above every category: `SettingsFileBar`, and what its press answered. */
export function SettingsFile({ settings, onOpen }: SettingsFileProps) {
  const [opening, setOpening] = useState(false);
  const [said, setSaid] = useState<string | null>(null);
  return (
    <SettingsFileBar
      path={settings?.path ?? null}
      refused={settings?.refused ?? null}
      opening={opening}
      {...(said === null ? {} : { said })}
      onOpen={() => {
        setOpening(true);
        setSaid(null);
        void onOpen().then((opened) => {
          setOpening(false);
          setSaid(whyNotOpened(opened));
        });
      }}
    />
  );
}

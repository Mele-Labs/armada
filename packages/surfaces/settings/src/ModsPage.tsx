// Every mod on this machine in one list: themes, layouts and trackers, one row each. A row offers what its kind
// has: the switch and Promote for all of them, and for a tracker its tried and valid marks and Try.

import { Card, CardContent, CardHeader, CardTitle, ModRow } from "@armada/components";
import type { ModRowProps } from "@armada/components";
import { useLayouts } from "@armada/shell";

import { useThemes } from "./theme-source";
import { useTrackers } from "./tracker-source";
import type { TrackerMod } from "./tracker-source";

type Row = { name: string; title: string; props: Omit<ModRowProps, "title"> };

/** A refusal or a failed try frames the row red, a mod switched off leaves it grey, a passed try on a valid mod green, and anything still to try amber. */
function standingOf(mod: TrackerMod): ModRowProps["standing"] {
  if (!mod.valid || mod.tried.state === "failed") return "refused";
  if (!mod.enabled) return "off";
  return mod.tried.state === "passed" ? "used" : "waiting";
}

export function ModsPage() {
  const themes = useThemes();
  const layouts = useLayouts();
  const trackers = useTrackers();
  const shared = (mod: { problem?: string; branch?: string }) => {
    const detail = mod.problem ?? mod.branch;
    return { ...(detail === undefined ? {} : { detail }), promoteDisabled: detail !== undefined };
  };
  const rows: Row[] = [
    ...themes.mods.map((mod) => ({
      name: mod.name,
      title: mod.title,
      props: { kind: "theme" as const, enabled: mod.enabled, onEnabled: (on: boolean) => themes.source.setEnabled(mod.name, on), onPromote: () => themes.source.promote(mod.name), ...shared(mod) },
    })),
    ...layouts.mods.map((mod) => ({
      name: mod.name,
      title: mod.title,
      props: { kind: "layout" as const, enabled: mod.enabled, onEnabled: (on: boolean) => layouts.source.setEnabled(mod.name, on), onPromote: () => layouts.source.promote(mod.name), ...shared(mod) },
    })),
    ...trackers.mods.map((mod) => ({
      name: mod.name,
      title: mod.title,
      props: {
        kind: "tracker" as const,
        tracker: { tried: mod.tried, valid: mod.valid },
        standing: standingOf(mod),
        enabled: mod.enabled,
        onEnabled: (on: boolean) => trackers.source.setEnabled(mod.name, on),
        onPromote: () => trackers.source.promote(mod.name),
        onTry: () => trackers.source.tryIt(mod.name),
        ...shared(mod),
      },
    })),
  ].sort((a, b) => a.name.localeCompare(b.name));
  if (rows.length === 0) return null;
  return (
    <Card>
      <CardHeader>
        <CardTitle>Mods</CardTitle>
      </CardHeader>
      <CardContent>
        {rows.map((row) => (
          <ModRow key={`${row.props.kind}:${row.name}`} title={row.title} {...row.props} />
        ))}
      </CardContent>
    </Card>
  );
}

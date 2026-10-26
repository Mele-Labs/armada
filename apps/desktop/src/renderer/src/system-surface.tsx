// System as one page: a sidebar of every entry under Kit, Fleet and Bridge, and the picked entry beside it.
// It replaces the strip of five pages that was under the title bar. Each entry draws what the page it came
// from drew; `system-entries.ts` says which, and which old surface opens which entry.
// A Kit item with a file is edited in the detail, through the same view the side panel uses. Show in Finder, for a folder with no file, goes to main, which opens a path only if a fresh inventory names it.

import { useEffect, useState, type ReactNode } from "react";

import { KitBrowser, KitModels, namesOf, SystemPage, SystemSidebar, useKept } from "@armada/components";
import type { KitKindId, KitPiecesProps, KitRef, KitSetupRead } from "@armada/components";
import type { ModelChoices, SettingsList } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import { addressReads, addressTyped, useKit, type KitSlice } from "@armada/screens/src/manifest-kit";
import { ModList, SETTINGS_SECTIONS, SettingsFile, SettingsSectionsView, taughtOf, useTaught, useThemes } from "@armada/settings";
import { Boundary, useLayouts, type BoundaryProps } from "@armada/shell";

import { entryOf, entryOfKind, foundBy, SYSTEM_ENTRIES, SYSTEM_GROUPS, type SystemEntryDef } from "./system-entries";
import { useKitModels } from "./kit-models";
import { openLink } from "./commands";
import { openLinkPanel } from "./link-drop";
import { askToCreateMod } from "./create-mod";
import { askToOpenSession } from "./open-session";
import { FileInline } from "./shared-panel/file";
import { openAddressInPanel, openInPanel, openMod } from "./shared-panel/store";
import { useSystemEntry } from "./system-entry";

export type SystemSurfaceProps = KitSlice & {
  /** The repository the rail has picked, as a person reads it, or `null` for All. Names Kit's Here control. */
  repository: string | null;
  settings: SettingsList | null;
  /** What Fleet answered to `list_models`, or `null` before it has. */
  modelChoices: ModelChoices | null;
  live: boolean;
  guarded: Pick<BoundaryProps, "bridge" | "onCopied">;
  onReadGuides: () => void;
  onOpenAccount: () => void;
  /** The Workflow creator, drawn under Kit › Workflows. */
  workflows: ReactNode;
  /** The guide catalogue, drawn under Bridge › Guides. */
  guides: ReactNode;
  /** A mark on an entry, by the old surface's name (`workflows`), and what its tooltip says. */
  warned: Readonly<Record<string, string>>;
};

/** A pull request opens in the side panel where one is hosted, and in the browser where not. */
const openPull = (address: string) => void (openAddressInPanel(address) || openLink(address));

/**
 * Kit's read, with Bridge's own mods as a kind of it: the themes and layouts in the mod folder, and the ones Armada
 * ships. The harness has no mods, so Fleet's read never carries the kind.
 */
function useWithMods(setup: KitSetupRead | undefined): KitSetupRead | undefined {
  const themes = useThemes();
  const layouts = useLayouts();
  if (setup === undefined) return undefined;
  const items = [
    ...themes.mods.map((mod) => ({ name: mod.name.replace(/^built-in:/, ""), source: mod.builtIn === true ? "Armada" : "", mod: { kind: "theme" as const, ...(mod.builtIn === true ? { builtIn: true } : {}) } })),
    ...layouts.mods.map((mod) => ({ name: mod.name, source: "", mod: { kind: "layout" as const } })),
  ];
  return { ...setup, kinds: [...setup.kinds.filter((one) => one.kind !== "mods"), { kind: "mods", read: { what: "read", items, unreadable: [] } }] };
}

export function SystemSurface({ repository, settings, modelChoices, live, guarded, onReadGuides, onOpenAccount, workflows, guides, warned, ...slice }: SystemSurfaceProps) {
  const read = useKit(slice);
  const kit = { ...read, setup: useWithMods(read.setup) };
  const [entry, setEntry] = useSystemEntry();
  const [query, setQuery] = useKept("system-search", "");
  const [picks, setPicks] = useState<Partial<Record<KitKindId, string>>>({});
  const [openRefused, setOpenRefused] = useState<string | undefined>(undefined);
  const open = (path: string, how: "editor" | "finder") =>
    void window.armada.openKitFile(path, how).then((done) => setOpenRefused(done.ok ? undefined : done.detail));
  const models = useKitModels(modelChoices, settings);
  const [model, setModel] = useState<string | undefined>(undefined);

  const servers = kit.kit?.servers.map((server) => ({
    name: server.name,
    address: addressReads(server.address),
    kind: server.address.transport,
    reachesByDefault: server.drones === "yes",
    ...(server.manifest === undefined ? {} : { here: server.manifest }),
    resolves: server.resolves,
  }));
  const names = (kind: KitKindId) => namesOf(kind, kit.setup, servers);
  const pieces: KitPiecesProps = {
    onAdd: () => openLinkPanel(""),
    onRemove: kit.pieces.onRemove,
    trashed: kit.pieces.trashed,
    onUndo: kit.pieces.onUndo,
    ...(kit.refused === null ? {} : { refused: said(kit.refused) }),
  };

  const found = new Map(SYSTEM_ENTRIES.map((one) => [one.id, foundBy(one, query, names, settings)] as const));
  // Words, Tracker and Fleet are listed once a teaching or a scaffold has made a mod of that kind, and not before.
  const taught = useTaught();
  const made = (id: string) =>
    id === "bridge:words" ? taughtOf(taught, "words").length > 0 : id === "bridge:tracker" ? taughtOf(taught, "tracker").length > 0 : id === "bridge:fleet" ? taughtOf(taught, "fleet").length > 0 : true;
  const visible = SYSTEM_ENTRIES.filter((one) => found.get(one.id) !== null && made(one.id));
  // A search that hides the entry last picked draws the first it found, without forgetting the pick.
  const shown: SystemEntryDef | undefined = visible.find((one) => one.id === entry) ?? visible[0];
  const narrow = shown !== undefined && found.get(shown.id) === "items" ? query : "";
  // The models page asks the harness again each time it opens, rather than draw what Fleet kept.
  const onModels = shown?.id === "kit:models";
  useEffect(() => {
    if (onModels) void window.armada.askModels();
  }, [onModels]);

  const groups = SYSTEM_GROUPS.map((group) => ({
    id: group.id,
    label: group.label,
    entries: visible
      .filter((one) => one.group === group.id)
      .map((one) => {
        const alert = one.id === "kit:workflows" ? warned["workflows"] : undefined;
        return { id: one.id, label: one.label, ...(alert === undefined ? {} : { alert }) };
      }),
    ...(group.id === "kit" ? { act: { label: "Add", name: "Add to Kit", onPress: () => openLinkPanel("") } } : {}),
  }));

  const jump = (to: KitRef) => {
    setPicks((was) => ({ ...was, [to.kind]: to.id }));
    setEntry(entryOfKind(to.kind));
  };

  const settingsFor = (def: SystemEntryDef): ReactNode => (
    <div className="armada-system__pane" data-col="detail">
      {def.id === "kit:harness" && kit.setup !== undefined ? (
        <p className="armada-system__heading">
          <span className="armada-system__heading-name">{kit.setup.harness}</span>
          <span className="armada-system__heading-home">{kit.setup.home}</span>
        </p>
      ) : null}
      {def.sections?.some((id) => SETTINGS_SECTIONS.find((one) => one.id === id)?.schema === true) ? (
        <SettingsFile
          settings={settings}
          // The file opens in the side panel where one is drawn, and in the machine's editor where none is.
          onOpen={async () => (settings?.path !== undefined && openInPanel({ kind: "file", path: settings.path, from: "settings" }) ? { ok: true } : window.armada.openSettingsFile())}
        />
      ) : null}
      <SettingsSectionsView
        ids={def.sections ?? []}
        settings={settings}
        live={live}
        onSaveSettings={(changes) => window.armada.saveSettings(changes)}
        onOpenAccount={onOpenAccount}
        onReadGuides={onReadGuides}
        {...(guarded.onCopied === undefined ? {} : { onCopied: guarded.onCopied })}
        onOpenPull={openPull}
        onOpenJob={askToOpenSession}
        onOpenMod={openMod}
        onCreateMod={askToCreateMod}
        under={{
          theme: (
            <>
              <ModList kind="theme" onOpenPull={openPull} onOpenJob={askToOpenSession} onOpenMod={openMod} />
              <ModList kind="tweak" title="Tweaks" onOpenMod={openMod} />
            </>
          ),
          layout: <ModList kind="layout" onOpenPull={openPull} onOpenJob={askToOpenSession} onOpenMod={openMod} />,
        }}
      />
      {def.id === "bridge:guides" ? <div className="armada-system__catalogue">{guides}</div> : null}
    </div>
  );

  const body = (def: SystemEntryDef): ReactNode => {
    if (def.kit !== undefined) {
      const kind = def.kit;
      return (
        <KitBrowser
          key={kind}
          kind={kind}
          label={def.label}
          setup={kit.setup}
          keep="system-kit"
          narrow={narrow}
          picked={picks[kind]}
          onPick={(id) => setPicks((was) => ({ ...was, [kind]: id }))}
          onJump={jump}
          onRemoveAllowed={kit.onRemoveAllowed}
          pieces={pieces}
          onOpenFile={(path) => {
            if (!openInPanel({ kind: "file", path, from: "kit" })) open(path, "editor");
          }}
          fileView={({ path, head }) => <FileInline key={path} path={path} from="kit" head={head} />}
          onShowInFinder={(path) => open(path, "finder")}
          {...(openRefused === undefined ? {} : { openRefused })}
          {...(kind !== "servers"
            ? {}
            : {
                servers: {
                  rows: servers,
                  here: repository,
                  ...(kit.refused === null ? {} : { refused: said(kit.refused) }),
                  onAdd: ({ name, kind: transport, address }: { name: string; kind: "stdio" | "http"; address: string }) => {
                    const typed = addressTyped(transport, address);
                    // Fleet refuses the same shapes and says why. What is refused here is an empty field.
                    if (typed !== null) kit.onAdd({ name, address: typed });
                  },
                  onForget: kit.onForget,
                  onKitReach: (name: string, reaches: boolean) => kit.onKitReach(name, reaches ? "yes" : "no"),
                  onHereReach: kit.onManifestReach,
                },
              })}
        />
      );
    }
    if (def.id === "kit:models") {
      return <KitModels models={models.models} keep="system-kit" picked={model} onPick={setModel} onOn={models.setOn} onChoose={models.choose} />;
    }
    if (def.id === "kit:workflows") {
      return (
        <div className="armada-system__pane" data-col="detail">
          {workflows}
        </div>
      );
    }
    return settingsFor(def);
  };

  return (
    <Boundary region="System" {...guarded}>
      <SystemPage side={<SystemSidebar groups={groups} value={shown?.id ?? entry} onValue={setEntry} query={query} onQuery={setQuery} />}>
        {shown === undefined ? null : body(entryOf(shown.id)!)}
      </SystemPage>
    </Boundary>
  );
}

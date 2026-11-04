// System as one page: a sidebar of every entry under Kit, Fleet and Bridge, and the picked entry beside it.
// It replaces the strip of five pages that was under the title bar. Each entry draws what the page it came
// from drew; `system-entries.ts` says which, and which old surface opens which entry.
// A Kit item with a file is edited in the detail, through the same view the side panel uses. Show in Finder, for a folder with no file, goes to main, which opens a path only if a fresh inventory names it.

import { useEffect, useState, type ReactNode } from "react";

import {
  KitBrowser,
  KitModels,
  namesOf,
  RepositorySwitches,
  SystemPage,
  SystemSidebar,
  useKept,
} from "@armada/components";
import type { KitKindId, KitPiecesProps, KitRef, KitSetupRead } from "@armada/components";
import type { ModelChoices, SettingsList } from "@armada/protocol";
import { said } from "@armada/screens/src/copy";
import {
  addressReads,
  addressTyped,
  useKit,
  type KitSlice,
} from "@armada/screens/src/manifest-kit";
import {
  ModList,
  SettingsSectionsView,
  taughtOf,
  useSettingsFile,
  useTaught,
  useThemes,
} from "@armada/settings";
import { Boundary, useLayouts, type BoundaryProps } from "@armada/shell";

import {
  entryOf,
  entryOfKind,
  foundBy,
  SYSTEM_ENTRIES,
  SYSTEM_GROUPS,
  type SystemEntryDef,
} from "./system-entries";
import { BrowserDefaults } from "./job-browser/BrowserDefaults";
import { useKitModels } from "./kit-models";
import { openLink } from "./commands";
import { openLinkPanel } from "./link-drop";
import { askToCreateMod } from "./create-mod";
import { askToOpenSession } from "./open-session";
import { FileInline } from "./shared-panel/file";
import { openAddressInPanel, openInPanel, openMod } from "./shared-panel/store";
import { usePaletteTable } from "./palette-table";
import { useSystemEntry } from "./system-entry";

export type SystemSurfaceProps = KitSlice & {
  /** The repository the rail has picked, as a person reads it, or `null` for All. Names Kit's Here control. */
  repository: string | null;
  /** Every repository Fleet serves, by Manifest id, for the switches set per repository. */
  repositories: readonly {
    id: string;
    name: string;
    /** Its `armada.yml` says `connectors: false`, which a repository's switch not set follows. */
    connectorsOff: boolean;
  }[];
  settings: SettingsList | null;
  /** What Fleet answered to `list_models`, or `null` before it has. */
  modelChoices: ModelChoices | null;
  live: boolean;
  guarded: Pick<BoundaryProps, "bridge" | "onCopied">;
  onReadGuides: () => void;
  onOpenAccount: () => void;
  /** The Workflow creator, drawn under Kit › Workflows. */
  workflows: ReactNode;
  /** Kit › Workspaces. Absent where the window holds no Workspaces, and the entry is not listed. */
  workspaces?: ReactNode;
  /** The guide catalogue, drawn under Bridge › Guides. */
  guides: ReactNode;
  /** A mark on an entry, by the old surface's name (`workflows`), and what its tooltip says. */
  warned: Readonly<Record<string, string>>;
};

/** The skills Jobs start without, by repository: `{ "<manifest id>": ["pptx"] }`. Drawn in a skill's detail, never as a row. */
const SKILLS_OFF_KEY = "tokens.skillsOff";
/** Whether each repository's Jobs start without the person's connectors, by Manifest id, `true` for off. One not in it follows its armada.yml. Drawn as System › Repository. */
const CONNECTORS_OFF_KEY = "tokens.connectorsOff";

/** A Json setting's value as a list of text, or none. */
const listOf = (value: unknown): string[] => (Array.isArray(value) ? value.map(String) : []);

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
    ...themes.mods.map((mod) => ({
      name: mod.name.replace(/^built-in:/, ""),
      source: mod.builtIn === true ? "Armada" : "",
      mod: {
        kind: "theme" as const,
        ...(mod.builtIn === true
          ? {
              builtIn: true,
            }
          : {}),
      },
    })),
    ...layouts.mods.map((mod) => ({
      name: mod.name,
      source: "",
      mod: {
        kind: "layout" as const,
      },
    })),
  ];
  return {
    ...setup,
    kinds: [
      ...setup.kinds.filter((one) => one.kind !== "mods"),
      {
        kind: "mods",
        read: {
          what: "read",
          items,
          unreadable: [],
        },
      },
    ],
  };
}

export function SystemSurface({
  repository,
  repositories,
  settings,
  modelChoices,
  live,
  guarded,
  onReadGuides,
  onOpenAccount,
  workflows,
  workspaces,
  guides,
  warned,
  ...slice
}: SystemSurfaceProps) {
  const palette = usePaletteTable();
  const read = useKit(slice);
  const kit = {
    ...read,
    setup: useWithMods(read.setup),
  };
  const [entry, setEntry] = useSystemEntry();
  const [query, setQuery] = useKept("system-search", "");
  const [picks, setPicks] = useState<Partial<Record<KitKindId, string>>>({});
  const [openRefused, setOpenRefused] = useState<string | undefined>(undefined);
  const open = (path: string, how: "editor" | "finder") =>
    void window.armada
      .openKitFile(path, how)
      .then((done) => setOpenRefused(done.ok ? undefined : done.detail));
  const models = useKitModels(modelChoices, settings);
  const [model, setModel] = useState<string | undefined>(undefined);

  const servers = kit.kit?.servers.map((server) => ({
    name: server.name,
    address: addressReads(server.address),
    kind: server.address.transport,
    reachesByDefault: server.drones === "yes",
    ...(server.manifest === undefined
      ? {}
      : {
          here: server.manifest,
        }),
    resolves: server.resolves,
  }));
  const names = (kind: KitKindId) => namesOf(kind, kit.setup, servers);
  // Both are set per repository and saved back whole, once Fleet serves them; the rail's pick changes neither.
  const skillsOff = settings?.settings.find((one) => one.key === SKILLS_OFF_KEY);
  const offBy: Record<string, readonly string[]> =
    typeof skillsOff?.value === "object" && skillsOff.value !== null && !Array.isArray(skillsOff.value)
      ? Object.fromEntries(
          Object.entries(skillsOff.value).map(([id, names]) => [
            id,
            listOf(names),
          ]),
        )
      : {};
  const jobs =
    skillsOff === undefined
      ? undefined
      : {
          repositories,
          off: offBy,
          onSwitch: (id: string, skill: string, on: boolean) => {
            const was = offBy[id] ?? [];
            const next = on ? was.filter((one) => one !== skill) : [...was, skill];
            const { [id]: _gone, ...rest } = offBy;
            const all =
              next.length === 0
                ? rest
                : {
                    ...rest,
                    [id]: next,
                  };
            void window.armada.saveSettings({
              [SKILLS_OFF_KEY]: Object.keys(all).length === 0 ? null : all,
            });
          },
        };
  const connectorsOff = settings?.settings.find((one) => one.key === CONNECTORS_OFF_KEY);
  // A value that is not a map of true and false reads as nothing set, so each repository follows its armada.yml.
  const setOff: Record<string, boolean> =
    typeof connectorsOff?.value === "object" && connectorsOff.value !== null && !Array.isArray(connectorsOff.value)
      ? Object.fromEntries(
          Object.entries(connectorsOff.value).flatMap(([id, off]) =>
            typeof off === "boolean"
              ? [
                  [
                    id,
                    off,
                  ],
                ]
              : [],
          ),
        )
      : {};
  const connectors =
    connectorsOff === undefined
      ? undefined
      : {
          off: (id: string, armadaYml: boolean) => setOff[id] ?? armadaYml,
          onSwitch: (id: string, armadaYml: boolean, on: boolean) => {
            // Back to what armada.yml says is no choice made, so the entry goes and the repository follows it again.
            const { [id]: _gone, ...rest } = setOff;
            const all = on === !armadaYml ? rest : { ...rest, [id]: !on };
            void window.armada.saveSettings({
              [CONNECTORS_OFF_KEY]: Object.keys(all).length === 0 ? null : all,
            });
          },
        };
  const pieces: KitPiecesProps = {
    onAdd: () => openLinkPanel(""),
    onRemove: kit.pieces.onRemove,
    trashed: kit.pieces.trashed,
    onUndo: kit.pieces.onUndo,
    ...(kit.refused === null
      ? {}
      : {
          refused: said(kit.refused),
        }),
  };

  const found = new Map(
    SYSTEM_ENTRIES.map(
      (one) =>
        [
          one.id,
          foundBy(one, query, names, settings),
        ] as const,
    ),
  );
  // Words, Tracker and Fleet are listed once a teaching or a scaffold has made a mod of that kind, and not before.
  const taught = useTaught();
  const made = (id: string) =>
    id === "bridge:words"
      ? taughtOf(taught, "words").length > 0
      : id === "bridge:tracker"
        ? taughtOf(taught, "tracker").length > 0
        : id === "bridge:fleet"
          ? taughtOf(taught, "fleet").length > 0
          : id === "fleet:repository"
            ? connectors !== undefined
            : id === "kit:workspaces"
              ? workspaces !== undefined
              : true;
  const visible = SYSTEM_ENTRIES.filter((one) => found.get(one.id) !== null && made(one.id));
  // A search that hides the entry last picked draws the first it found, without forgetting the pick.
  const shown: SystemEntryDef | undefined = visible.find((one) => one.id === entry) ?? visible[0];
  const narrow = shown !== undefined && found.get(shown.id) === "items" ? query : "";
  // The models page asks the harness again each time it opens, rather than draw what Fleet kept.
  const onModels = shown?.id === "kit:models";
  useEffect(() => {
    if (onModels) void window.armada.askModels();
  }, [
    onModels,
  ]);

  const groups = SYSTEM_GROUPS.map((group) => ({
    id: group.id,
    label: group.label,
    entries: visible
      .filter((one) => one.group === group.id)
      .map((one) => {
        const alert = one.id === "kit:workflows" ? warned["workflows"] : undefined;
        return {
          id: one.id,
          label: one.label,
          ...(alert === undefined
            ? {}
            : {
                alert,
              }),
        };
      }),
    ...(group.id === "kit"
      ? {
          act: {
            label: "Add",
            name: "Add to Kit",
            onPress: () => openLinkPanel(""),
          },
        }
      : {}),
  }));

  const jump = (to: KitRef) => {
    setPicks((was) => ({
      ...was,
      [to.kind]: to.id,
    }));
    setEntry(entryOfKind(to.kind));
  };

  // The file opens in the side panel where one is drawn, and in the machine's editor where none is.
  const settingsFile = useSettingsFile({
    settings,
    onOpen: async () =>
      settings?.path !== undefined &&
      openInPanel({
        kind: "file",
        path: settings.path,
        from: "settings",
      })
        ? {
            ok: true,
          }
        : window.armada.openSettingsFile(),
  });
  const settingsFor = (def: SystemEntryDef): ReactNode => (
    <div className="armada-system__pane" data-col="detail">
      {def.id === "kit:harness" && kit.setup !== undefined ? (
        <p className="armada-system__heading">
          <span className="armada-system__heading-name">{kit.setup.harness}</span>
          <span className="armada-system__heading-home">{kit.setup.home}</span>
        </p>
      ) : null}
      <SettingsSectionsView
        ids={def.sections ?? []}
        file={settingsFile}
        settings={settings}
        live={live}
        onSaveSettings={(changes) => window.armada.saveSettings(changes)}
        onOpenAccount={onOpenAccount}
        onReadGuides={onReadGuides}
        palette={palette}
        {...(guarded.onCopied === undefined
          ? {}
          : {
              onCopied: guarded.onCopied,
            })}
        onOpenPull={openPull}
        onOpenJob={askToOpenSession}
        onOpenMod={openMod}
        onCreateMod={askToCreateMod}
        under={{
          theme: (
            <>
              <ModList
                kind="theme"
                onOpenPull={openPull}
                onOpenJob={askToOpenSession}
                onOpenMod={openMod}
              />
              <ModList kind="tweak" title="Tweaks" onOpenMod={openMod} />
            </>
          ),
          layout: (
            <ModList
              kind="layout"
              onOpenPull={openPull}
              onOpenJob={askToOpenSession}
              onOpenMod={openMod}
            />
          ),
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
          onPick={(id) =>
            setPicks((was) => ({
              ...was,
              [kind]: id,
            }))
          }
          onJump={jump}
          onRemoveAllowed={kit.onRemoveAllowed}
          pieces={pieces}
          onOpenFile={(path) => {
            if (
              !openInPanel({
                kind: "file",
                path,
                from: "kit",
              })
            )
              open(path, "editor");
          }}
          fileView={({ path, head }) => (
            <FileInline key={path} path={path} from="kit" head={head} />
          )}
          onShowInFinder={(path) => open(path, "finder")}
          {...(openRefused === undefined
            ? {}
            : {
                openRefused,
              })}
          {...(jobs === undefined
            ? {}
            : {
                jobs,
              })}
          {...(kind !== "servers"
            ? {}
            : {
                servers: {
                  rows: servers,
                  here: repository,
                  ...(kit.refused === null
                    ? {}
                    : {
                        refused: said(kit.refused),
                      }),
                  onAdd: ({
                    name,
                    kind: transport,
                    address,
                  }: {
                    name: string;
                    kind: "stdio" | "http";
                    address: string;
                  }) => {
                    const typed = addressTyped(transport, address);
                    // Fleet refuses the same shapes and says why. What is refused here is an empty field.
                    if (typed !== null)
                      kit.onAdd({
                        name,
                        address: typed,
                      });
                  },
                  onForget: kit.onForget,
                  onKitReach: (name: string, reaches: boolean) =>
                    kit.onKitReach(name, reaches ? "yes" : "no"),
                  onHereReach: kit.onManifestReach,
                },
              })}
        />
      );
    }
    if (def.id === "kit:models") {
      return (
        <KitModels
          models={models.models}
          keep="system-kit"
          picked={model}
          onPick={setModel}
          onOn={models.setOn}
          onChoose={models.choose}
        />
      );
    }
    if (def.id === "fleet:repository" && connectors !== undefined) {
      return (
        <div className="armada-system__pane" data-col="detail">
          <div className="armada-system__section">
            <RepositorySwitches
              title="Connectors"
              help="Off, Jobs in that repository start without the connectors on your claude.ai account, such as Notion, Drive and Slack. Servers in the repository's own .mcp.json are kept, and sessions started outside Armada are unaffected. A switch you have not set follows the repository's armada.yml."
              rows={repositories.map((one) => ({
                id: one.id,
                name: one.name,
                on: !connectors.off(one.id, one.connectorsOff),
              }))}
              switchLabel={(name) => `Connectors in ${name}`}
              disabled={!live}
              onSwitch={(id, on) =>
                connectors.onSwitch(id, repositories.find((one) => one.id === id)?.connectorsOff ?? false, on)
              }
            />
          </div>
        </div>
      );
    }
    if (def.id === "bridge:browser") {
      return (
        <div className="armada-system__pane" data-col="detail">
          <div className="armada-system__section">
            <BrowserDefaults />
          </div>
        </div>
      );
    }
    if (def.id === "kit:workflows") {
      return workflows;
    }
    if (def.id === "kit:workspaces") {
      return workspaces;
    }
    return settingsFor(def);
  };

  return (
    <Boundary region="System" {...guarded}>
      <SystemPage
        side={
          <SystemSidebar
            groups={groups}
            value={shown?.id ?? entry}
            onValue={setEntry}
            query={query}
            onQuery={setQuery}
          />
        }
      >
        {shown === undefined ? null : body(entryOf(shown.id)!)}
      </SystemPage>
    </Boundary>
  );
}

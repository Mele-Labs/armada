// A mod in the side panel: what it is and where it lives, whether it is valid, the Job that made it, its switch, Promote where the row offers it,
// and Change it, which starts an empty Mod Job aimed at what the mod is. Read from Fleet's mod list, so the switch and the facts follow it live.

import { Box } from "lucide-react";
import { Button, MOD_KIND_SAID, ModKindMark, Switch } from "@armada/components";
import type { ModKind } from "@armada/protocol";
import { useTaught, useThemes } from "@armada/settings";
import { useLayouts } from "@armada/shell";

import { modTitle, setModEnabled } from "../mods-made";
import { useModList } from "../mod-list";
import { startEmptyModJob } from "../mod-job";
import { askToOpenSession } from "../open-session";
import { useSessions } from "../sessions-draft";
import { useBack } from "./panels";
import { SidePanel } from "./side";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

type Item = Extract<PanelItem, { kind: "mod" }>;

/** Where a mod's folder is made on this machine. */
const MODS_FOLDER = "~/Library/Application Support/Armada/mods";

/** What a Mod Job aims at to change a mod of this kind: a tweak is laid over a theme, so it is changed as one. */
const AIM = { theme: "theme", tweak: "theme", layout: "layout", words: "words", fleet: "fleet", tracker: "tracker" } as const;

/** The kind as it reads in the words the box opens on. */
const WORD = { theme: "theme", tweak: "theme", layout: "layout", words: "words", fleet: "Fleet rule", tracker: "tracker" } as const satisfies Record<ModKind, string>;

/** The words an empty Mod Job's box opens on to change this mod, the caret after the colon. */
export const changeWords = (title: string, kind: ModKind): string => `Change the ${title} ${WORD[kind]}: `;

export function ModPanel({ item }: { item: Item }) {
  const mods = useModList();
  const sessions = useSessions();
  const themes = useThemes();
  const layouts = useLayouts();
  const taught = useTaught();
  const back = useBack();
  const row = mods.find((one) => one.name === item.name);
  const kind = row?.kind ?? item.modKind;
  const title = item.title === "" ? modTitle(item.name) : item.title;
  const builtIn = row?.origin === "built_in";
  const maker = row?.made_by === undefined ? undefined : sessions.find((one) => one.id === row.made_by);
  // Promote is the Mods page's own act, on the source its kind comes from, and is offered where the page offers it.
  const promote = kind === "theme" ? themes.source.promote : kind === "layout" ? layouts.source.promote : taught.source.promote;
  const held = ([...themes.mods, ...layouts.mods, ...taught.mods] as readonly { name: string; branch?: string; pull?: unknown }[]).find((one) => one.name === item.name);
  const promotable = row !== undefined && !builtIn && kind !== "tweak";
  const promoteOff = row?.valid === false || row?.pull_request !== undefined || held?.branch !== undefined || held?.pull !== undefined;
  const tabs = [{ id: "details", label: "Details" }];
  return (
    <SidePanel
      title={`Mod ${title}`}
      heading={title}
      meta={
        <>
          <ModKindMark kind={kind} />
          <span className="armada-pr-panel__who">{MOD_KIND_SAID[kind]}</span>
        </>
      }
      acts={
        <>
          {row === undefined ? null : (
            <Switch checked={row.enabled} onChange={(event) => setModEnabled(row.name, event.target.checked)}>
              Switched on
            </Switch>
          )}
          <Button variant="primary" size="sm" onClick={() => void startEmptyModJob({ aim: AIM[kind], said: changeWords(title, kind) }).then(closePanel)}>
            Change it
          </Button>
          {promotable ? (
            <Button variant="secondary" size="sm" disabled={promoteOff} onClick={() => promote(item.name)}>
              Promote
            </Button>
          ) : null}
        </>
      }
      tabs={tabs}
      tab="details"
      onTab={() => undefined}
      back={back}
      onClose={closePanel}
    >
      <section className="side-mod" aria-label="Details">
        {row?.description === undefined ? null : <p className="side-mod__description">{row.description}</p>}
        <dl className="side-facts">
          <div className="side-facts__row">
            <dt>Lives in</dt>
            <dd className={builtIn ? undefined : "mono"}>{builtIn ? "Built in" : `${MODS_FOLDER}/${item.name}`}</dd>
          </div>
          {row?.version === undefined ? null : (
            <div className="side-facts__row">
              <dt>Version</dt>
              <dd className="mono">{row.version}</dd>
            </div>
          )}
          {row === undefined ? null : (
            <div className="side-facts__row">
              <dt>Valid</dt>
              <dd>{row.valid ? "Valid" : (row.reason ?? "Not valid")}</dd>
            </div>
          )}
          {row?.made_by === undefined ? null : (
            <div className="side-facts__row">
              <dt>Made by</dt>
              <dd>
                <Button variant="ghost" size="sm" onClick={() => askToOpenSession(row.made_by!)}>
                  <Box size={12} strokeWidth={2} aria-hidden />
                  {maker?.title ?? "Job"}
                </Button>
              </dd>
            </div>
          )}
        </dl>
      </section>
    </SidePanel>
  );
}

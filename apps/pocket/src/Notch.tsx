// The bar over every screen, as native's title bar draws it narrow (`crates/bridge-native/src/titlebar/mod.rs`): the mark,
// a notch holding the place and search, then the plan's rings, the Fleet dot and the menu. On the narrow bar native's
// project chip, Operator and dispatch move into the menu, and so they do here.

import { useEffect, useState, type ReactNode } from "react";
import { Bot, Box, ChevronLeft, FileText, Globe, Layers, Menu, Plus, Search } from "lucide-react";
import { ArmadaMark } from "@armada/brand";
import { Tooltip } from "@armada/components";

import { getJson } from "./client";
import type { Mac } from "./fleet";
import type { PlanUsageBody } from "./gateway";
import { go, pairPath } from "./router";

const LIMITS = [
  [
    "five_hour",
    "5-hour limit",
  ],
  [
    "seven_day",
    "Weekly limit",
  ],
] as const;

/** The newest reading any paired Mac holds, read again whenever the Macs' lists change. */
function usePlanUsage(macs: readonly Mac[], at: number): PlanUsageBody | undefined {
  const [usage, setUsage] = useState<PlanUsageBody | undefined>(undefined);
  const paired = macs.filter((mac) => mac.paired);
  useEffect(() => {
    let gone = false;
    void Promise.all(
      paired.map((mac) =>
        getJson<PlanUsageBody>(mac.machine, "/api/plan_usage").catch(() => undefined),
      ),
    ).then((each) => {
      if (gone) return;
      const read = each.filter(
        (one): one is PlanUsageBody => one !== undefined && one.read_at !== undefined,
      );
      setUsage(read.sort((a, b) => Date.parse(b.read_at!) - Date.parse(a.read_at!))[0]);
    });
    return () => {
      gone = true;
    };
  }, [
    paired.map((mac) => mac.machine.origin).join(" "),
    at,
  ]);
  return usage;
}

/** Native's two rings (`titlebar/usage.rs`): the 5-hour limit outside, the weekly inside, each in its band's tone. */
function Rings({ usage }: { usage: PlanUsageBody | undefined }) {
  const drawn = LIMITS.flatMap(([kind, name]) => {
    const one = usage?.windows.find((window) => window.kind === kind);
    return one === undefined
      ? []
      : [
          {
            name,
            percent: Math.round(one.percent_used),
            band: one.band ?? "normal",
          },
        ];
  });
  if (drawn.length === 0) return null;
  const said = drawn.map((one) => `${one.name} ${one.percent}%`).join(", ");
  return (
    <Tooltip label={said}>
      <span className="pk-rings" role="img" aria-label={said}>
        <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden="true">
          {drawn.map((one, at) => {
            const r = 9 - at * 4;
            const run = 2 * Math.PI * r;
            return (
              <g key={one.name} data-band={one.band}>
                <circle className="pk-rings__track" cx="11" cy="11" r={r} />
                <circle
                  className="pk-rings__fill"
                  cx="11"
                  cy="11"
                  r={r}
                  strokeDasharray={`${(run * Math.min(one.percent, 100)) / 100} ${run}`}
                />
              </g>
            );
          })}
        </svg>
      </span>
    </Tooltip>
  );
}

/** Native's Fleet dot, read across the paired Macs: every one answering, some not, or none. */
function FleetDot({ macs }: { macs: readonly Mac[] }) {
  const down = macs.filter((mac) => !mac.paired || mac.error !== undefined);
  const tone =
    macs.length === 0
      ? "unknown"
      : down.length === 0
        ? "running"
        : down.length < macs.length
          ? "some"
          : "down";
  const name = (mac: Mac) => mac.machine.name || new URL(mac.machine.origin).hostname;
  const said =
    tone === "running"
      ? `Fleet: running on ${macs.map(name).join(", ")}`
      : tone === "unknown"
        ? "Fleet: no Mac paired"
        : `Fleet: not answering on ${down.map(name).join(", ")}`;
  return (
    <Tooltip label={said}>
      <span className="pk-fleet" role="img" aria-label={said}>
        <span className="pk-fleet__dot" data-tone={tone} />
      </span>
    </Tooltip>
  );
}

/** A Mac's hue on its dot, by its place among the paired Macs, as native's `settings/macs.rs` gives them. */
export const MAC_HUES = [
  "blue",
  "teal",
  "green",
  "orange",
] as const;

function Item({
  icon,
  children,
  end,
  on,
  onPress,
}: {
  icon: ReactNode;
  children: ReactNode;
  end?: ReactNode;
  on?: boolean;
  onPress: () => void;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      className="pk-menu__item"
      data-on={on || undefined}
      onClick={onPress}
    >
      <span className="pk-menu__glyph">{icon}</span>
      <span className="pk-menu__words">{children}</span>
      {end === undefined ? null : <span className="pk-menu__end">{end}</span>}
    </button>
  );
}

export type Notched = {
  macs: readonly Mac[];
  /** When the lists were read, so the rings are read again with them. */
  at: number;
  /** The place the notch names: the Jobs list, or a Job, whose notch takes him back to the list. */
  place: "jobs" | "job";
  onSearch?: () => void;
  /** The projects the list groups under, and the one it shows; absent off the list. */
  projects?: {
    names: readonly string[];
    shown: string | undefined;
    onShow: (name: string | undefined) => void;
    onDrafts: () => void;
  };
};

export function Notch({ macs, at, place, onSearch, projects }: Notched) {
  const usage = usePlanUsage(macs, at);
  const [open, setOpen] = useState(false);
  useEffect(() => {
    if (!open) return;
    const away = (event: KeyboardEvent) => event.key === "Escape" && setOpen(false);
    window.addEventListener("keydown", away);
    return () => window.removeEventListener("keydown", away);
  }, [
    open,
  ]);
  const press = (act: () => void) => () => {
    setOpen(false);
    act();
  };
  const name = (mac: Mac) => mac.machine.name || new URL(mac.machine.origin).hostname;
  return (
    <header className="pk-notch-bar">
      <span className="pk-notch-bar__mark" aria-hidden="true">
        <ArmadaMark size={20} />
      </span>
      <nav className="pk-notch" aria-label="Places">
        <button
          type="button"
          className="pk-notch__place"
          data-here={place === "jobs" || undefined}
          aria-label={place === "jobs" ? "Jobs" : "Back to Jobs"}
          onClick={() => go("/")}
        >
          {place === "jobs" ? (
            <Box size={14} aria-hidden="true" />
          ) : (
            <ChevronLeft size={14} aria-hidden="true" />
          )}
          Jobs
        </button>
        {onSearch === undefined ? null : (
          <Tooltip label="Search Jobs">
            <button
              type="button"
              className="pk-notch__search"
              aria-label="Search Jobs"
              onClick={onSearch}
            >
              <Search size={16} aria-hidden="true" />
            </button>
          </Tooltip>
        )}
      </nav>
      <span className="pk-notch-bar__end">
        <Rings usage={usage} />
        <FleetDot macs={macs} />
        <Tooltip label="Menu">
          <button
            type="button"
            className="pk-notch-bar__menu"
            aria-label="Menu"
            aria-haspopup="menu"
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            <Menu size={16} aria-hidden="true" />
          </button>
        </Tooltip>
      </span>
      {!open ? null : (
        <>
          <button
            type="button"
            className="pk-menu__away"
            aria-label="Close the menu"
            onClick={() => setOpen(false)}
          />
          <div className="pk-menu" role="menu" aria-label="Menu">
            <p className="pk-menu__head">Macs</p>
            {macs.map((mac, at) => {
              const down = !mac.paired || mac.error !== undefined;
              return (
                <Item
                  key={mac.machine.origin}
                  icon={<span className="pk-mac" data-hue={MAC_HUES[at % MAC_HUES.length]} />}
                  end={mac.paired ? (down ? "Not answering" : "Running") : "Not paired"}
                  onPress={press(() => (mac.paired ? undefined : go(pairPath(mac.machine.origin))))}
                >
                  {name(mac)}
                </Item>
              );
            })}
            <Item
              icon={<Plus size={16} aria-hidden="true" />}
              onPress={press(() => go(pairPath()))}
            >
              Pair a Mac
            </Item>
            {projects === undefined || projects.names.length < 2 ? null : (
              <>
                <hr className="pk-menu__rule" />
                <p className="pk-menu__head">Projects</p>
                <Item
                  icon={<Globe size={16} aria-hidden="true" />}
                  on={projects.shown === undefined}
                  onPress={press(() => projects.onShow(undefined))}
                >
                  All
                </Item>
                {projects.names.map((one) => (
                  <Item
                    key={one}
                    icon={<Layers size={16} aria-hidden="true" />}
                    on={projects.shown === one}
                    onPress={press(() => projects.onShow(one))}
                  >
                    {one}
                  </Item>
                ))}
              </>
            )}
            <hr className="pk-menu__rule" />
            <Item
              icon={<Plus size={16} aria-hidden="true" />}
              onPress={press(() => go("/dispatch"))}
            >
              New Job
            </Item>
            <Item
              icon={<Bot size={16} aria-hidden="true" />}
              onPress={press(() => go("/operator"))}
            >
              Operator
            </Item>
            {projects === undefined ? null : (
              <Item
                icon={<FileText size={16} aria-hidden="true" />}
                onPress={press(projects.onDrafts)}
              >
                Drafts
              </Item>
            )}
          </div>
        </>
      )}
    </header>
  );
}

// The Cockpit as the app, in one familiar pattern: a workspace tab bar, as a browser or an IDE has one. The
// places are pinned tabs (Cockpit, Jobs, Sessions, Checks, Merge line, Bays); opening a Job or a Session from
// anywhere opens it as a tab of its own to the right, and opening it again focuses that tab. A tab is the
// app's own state read back, so the real pages draw unmodified under it; what a tab remembers (the Cockpit's
// selection and Peek, a page's scroll) is kept here, so closing a tab returns to the one it was reached from
// in exactly the view that was left. Under the bar every tab draws one title line, and the Cockpit's holds the
// roll call. The calls wait in a row above the page, never over it. Mock only. proto/cockpit-three-directions.

import { useEffect, useMemo, useRef, useState } from "react";
import { ExternalLink, Hammer, Reply, Square, Terminal, X as Close } from "lucide-react";
import { Button, Kbd, Tooltip } from "@armada/components";
import { SURFACE } from "@armada/shell";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import type { AppSeamProps } from "../../cockpit/app-seam";
import type { CockpitProps } from "../../cockpit/body-seam";
import { Horizon } from "../../cockpit/Cockpit";
import { useSessions } from "../../sessions-draft";
import { useCalls } from "./calls";
import { FixAct, IconAct, KeySheet, MarkGlyph, PeekPop, PrChip, STATION_GLYPH, StepsBar, useShipActs, useShipUi, type Acts, type ShipUi } from "./shared";
import { useShip, type Ship, type Subject } from "./ship";
import {
  CHORD,
  PINNED,
  PLACE_NAME,
  arrive,
  askFocus,
  drop,
  exists,
  groupedOf,
  idOfPlace,
  isPinned,
  mainIsRed,
  needsOf,
  orderOf,
  pinnedOf,
  readyPrs,
  rememberScroll,
  returnFrom,
  rollOf,
  scrollOf,
  tabsNow,
  toneOf,
  useTabs,
  type Roll,
} from "./tabs-model";
import "./tabs.css";

const KEYS_MORE: readonly (readonly [readonly string[], string])[] = [
  [["1", "2", "3", "4", "5", "6"], "Cockpit, Jobs, Sessions, Checks, Merge line, Bays"],
  [["[", "]"], "Previous or next tab"],
  [["x", "⌘W"], "Close the tab (⌘W only on a Job: x is Kill there)"],
  [[",", "."], "Previous or next thing that needs you"],
  [["g", "a letter"], "j s c m b t r: Studios and Retros open as tabs"],
];

// ---- Marks -----------------------------------------------------------------------------------------

/** A place's mark: its own glyph, the shape of its state beside it, and its loudness in the colour. The tooltip names why. */
function RollMark({ roll, onPress, digit }: { roll: Roll; onPress?: () => void; digit?: string }) {
  const Glyph = STATION_GLYPH[roll.id];
  const Badge = roll.badge;
  const mark = (
    <Tooltip
      label={
        <span className="proto-why">
          {roll.lines.map((line) => (
            <span key={line}>{line}</span>
          ))}
        </span>
      }
    >
      <span className="tabs-mark" data-tone={roll.tone} role="img" aria-label={roll.lines.join("; ")}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
        {Badge === undefined ? null : <Badge size={12} strokeWidth={2.5} aria-hidden />}
      </span>
    </Tooltip>
  );
  return onPress === undefined ? (
    <span className="tabs-roll">{mark}</span>
  ) : (
    <button type="button" className="tabs-roll" onClick={onPress}>
      {mark}
      {digit === undefined ? null : <Kbd>{digit}</Kbd>}
    </button>
  );
}

// ---- The tab bar -----------------------------------------------------------------------------------

type TabView = { id: string; title: string; subject: Subject | undefined; pinned: boolean; digit?: string; red?: boolean };

function TabBar({ tabs, active, onPick, onClose }: { tabs: readonly TabView[]; active: string; onPick: (id: string) => void; onClose: (id: string) => void }) {
  return (
    <div className="tabs-bar" role="tablist" aria-label="Open tabs">
      {tabs.map((one, at) => (
        <div key={one.id} className="tabs-tab" data-active={one.id === active || undefined} data-pinned={one.pinned || undefined} data-first-subject={(!one.pinned && tabs[at - 1]?.pinned === true) || undefined}>
          <button type="button" role="tab" aria-selected={one.id === active} className="tabs-tab__press" onClick={() => onPick(one.id)}>
            {one.subject === undefined ? null : <MarkGlyph mark={{ ...one.subject.mark, tone: toneOf(one.subject) }} size={12} />}
            {one.red === true ? (
              <Tooltip label="Main is failing">
                <span className="tabs-mark" data-tone="bad" role="img" aria-label="Main is failing">
                  <Close size={12} strokeWidth={2.5} aria-hidden />
                </span>
              </Tooltip>
            ) : null}
            <span className="tabs-tab__title">{one.title}</span>
            {one.digit === undefined ? null : <Kbd>{one.digit}</Kbd>}
          </button>
          {one.pinned ? null : (
            <Tooltip label="Close">
              <button type="button" className="tabs-tab__close" aria-label={`Close ${one.title}`} onClick={() => onClose(one.id)}>
                <Close size={14} aria-hidden />
              </button>
            </Tooltip>
          )}
        </div>
      ))}
    </div>
  );
}

// ---- The title line --------------------------------------------------------------------------------

function TitleLine({ props, ship, active, subject, titleOf, onGo, acts, jump }: { props: AppSeamProps; ship: Ship; active: string; subject: Subject | undefined; titleOf: (id: string) => string; onGo: (id: string) => void; acts: Acts; jump: () => void }) {
  const scope = props.picked?.manifest?.repository ?? "All repositories";
  const rolled = rollOf(ship);
  const kind = active.startsWith("job:") ? "Job" : active.startsWith("session:") ? "Session" : undefined;
  const ready = readyPrs(ship).flatMap((pr) => {
    const owner = ship.subjects.find((one) => one.pr?.key === pr.key && one.mergeable);
    return owner === undefined ? [] : [{ pr, owner }];
  });
  return (
    <section className="tabs-title" aria-label="Title line">
      {subject === undefined ? null : <MarkGlyph mark={{ ...subject.mark, tone: toneOf(subject) }} size={16} />}
      <h2 className="tabs-title__name">{titleOf(active)}</h2>
      <span className="tabs-title__scope">{kind === undefined ? scope : `${kind} · ${subject?.repo ?? scope}`}</span>
      {active === "cockpit" ? (
        <span className="tabs-title__roll" role="group" aria-label="Roll call">
          {rolled.map((roll) => (
            <RollMark
              key={roll.id}
              roll={roll}
              {...(roll.id === "main" ? { onPress: jump } : roll.id === "merge" ? { onPress: () => onGo("merge"), digit: "5" } : roll.id === "bays" ? { onPress: () => onGo("bays"), digit: "6" } : {})}
            />
          ))}
        </span>
      ) : null}
      {active !== "merge"
        ? null
        : ready.map(({ pr, owner }) => (
            <Button key={pr.key} variant="secondary" size="sm" aria-label={`Merge #${pr.number}`} onClick={() => acts.merge(owner)}>
              Merge #{pr.number}
              <Kbd>m</Kbd>
            </Button>
          ))}
      <Tooltip label="Keys">
        <Kbd className="tabs-title__keys">?</Kbd>
      </Tooltip>
    </section>
  );
}

// ---- The chrome ------------------------------------------------------------------------------------

/** What the shell draws at the top of its mount on every page: the tab bar, the title line, the calls. */
export function TabsChrome(props: AppSeamProps) {
  const ship = useShip(props);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const calls = useCalls(props, ship);
  const sessions = useSessions();
  const tabs = useTabs();
  const latest = useRef(props);
  latest.current = props;
  const place = idOfPlace(props);
  const placeRef = useRef(place);
  placeRef.current = place;

  const titleOf = (id: string): string => {
    const pin = pinnedOf(id);
    if (pin !== undefined) return pin.name;
    if (id.startsWith("job:")) return props.state.jobs.find((one) => one.id === id.slice(4))?.title ?? "Job";
    if (id.startsWith("session:")) return sessions.find((one) => one.id === id.slice(8))?.title ?? id.slice(8);
    return PLACE_NAME[id.slice(6)] ?? id.slice(6);
  };
  const ids = [...PINNED.map((one) => one.id), ...tabs.opened, ...(exists(place) ? [] : [place])];
  const views: TabView[] = ids.map((id) => {
    const pin = pinnedOf(id);
    return { id, title: titleOf(id), subject: ship.subjects.find((one) => one.key === id), pinned: pin !== undefined, ...(pin === undefined ? {} : { digit: pin.digit }), ...(id === "cockpit" && id !== place && mainIsRed(ship) ? { red: true } : {}) };
  });

  // Going to a tab drives the app's own state; the tab bar reads it back, so there is one source of where he is.
  const lastGo = useRef(0);
  const go = (id: string) => {
    const p = latest.current;
    if (id === placeRef.current) return;
    lastGo.current = Date.now();
    const pin = pinnedOf(id);
    if (pin !== undefined) p.goTo(pin.surface);
    else if (id.startsWith("job:")) (p.goTo(SURFACE.overview), p.onOpen(id.slice(4)));
    else if (id.startsWith("session:")) p.onOpenSession(id.slice(8));
    else p.goTo(id.slice(6));
  };
  const close = (id: string) => {
    if (isPinned(id)) return;
    const back = returnFrom(id);
    if (id === placeRef.current) go(back);
    drop(id);
  };

  // The app opened something, or went somewhere: that is the tab in front, opened if it was not.
  const was = useRef(place);
  const escaped = useRef(0);
  useEffect(() => {
    const on = (event: KeyboardEvent) => {
      if (event.key === "Escape") escaped.current = Date.now();
    };
    window.addEventListener("keydown", on, true);
    return () => window.removeEventListener("keydown", on, true);
  }, []);
  useEffect(() => {
    const before = was.current;
    was.current = place;
    // A Job that closed itself on Escape has been closed as a tab: back to where it was reached from.
    if (before !== place && before.startsWith("job:") && place === "cockpit" && Date.now() - lastGo.current > 400 && Date.now() - escaped.current < 500 && exists(before)) {
      const back = returnFrom(before);
      drop(before);
      if (back !== "cockpit") return go(back);
    }
    arrive(place);
    restoreScroll(place);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [place]);
  // `?` lists this window's keys. The app's own Keys drawer answers it too, so it is taken before the app hears it.
  const sheet = useRef(ui);
  sheet.current = ui;
  useEffect(() => {
    const on = (event: KeyboardEvent) => {
      if (event.key !== "?" || event.metaKey || event.ctrlKey || event.altKey || holdsText(event.target)) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      sheet.current.setSheet(!sheet.current.sheet);
    };
    window.addEventListener("keydown", on, true);
    return () => window.removeEventListener("keydown", on, true);
  }, []);
  // What a page was scrolled to is kept for the tab, by the scroller's own signature.
  useEffect(() => {
    const on = (event: Event) => {
      const target = event.target;
      if (!(target instanceof HTMLElement) || target.closest(".tabs-chrome, .proto-peek") !== null) return;
      const key = scrollerKey(target);
      if (key !== undefined) rememberScroll(placeRef.current, key, target.scrollTop);
    };
    window.addEventListener("scroll", on, true);
    return () => window.removeEventListener("scroll", on, true);
  }, []);

  // The Cockpit's rows in the order j and k walk them.
  const order = useMemo(() => orderOf(ship), [ship]);
  const current = order.find((one) => one.key === ui.selected) ?? (ui.selected === undefined ? order[0] : undefined);
  const onCockpit = place === "cockpit";
  const subjectOf = (event: KeyboardEvent): Subject | undefined => {
    const aimed = (event.target as HTMLElement | null)?.closest<HTMLElement>("[data-subject],[data-unit]");
    if (aimed?.dataset.subject !== undefined) return ship.subjects.find((one) => one.key === aimed.dataset.subject);
    if (aimed?.dataset.unit !== undefined) return ship.subjects.find((one) => one.pr?.key === aimed.dataset.unit);
    return current;
  };
  const hot = pointed ?? current?.pr?.number;
  const reply = (subject: Subject) => {
    ui.select(subject.key);
    ui.setReplying(undefined);
    ui.setPeeking(subject.key);
    // The Peek's own box takes the words, as it does on the Session's page.
    window.setTimeout(() => document.querySelector<HTMLElement>('.proto-peek__foot [role="textbox"], .proto-peek__foot textarea')?.focus(), 120);
  };
  const needs = (by: 1 | -1) => {
    const list = needsOf(ship);
    if (list.length === 0) return;
    // From another tab the walk starts at the top, the most urgent; on the Cockpit it goes on from where he is.
    const at = place === "cockpit" ? list.findIndex((one) => one.key === ui.selected) : -1;
    const next = list[at < 0 ? (by > 0 ? 0 : list.length - 1) : (at + by + list.length) % list.length]!;
    ui.select(next.key);
    askFocus(next.key);
    go("cockpit");
  };

  const armed = useRef<number | undefined>(undefined);
  const chording = useRef(false);
  useListKeydown((event) => {
    const mod = event.metaKey || event.ctrlKey;
    if (mod && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "w") {
      if (!isPinned(place)) {
        event.preventDefault();
        close(place);
      }
      return;
    }
    if (mod || event.altKey || holdsText(event.target)) return;
    if (chording.current) {
      window.clearTimeout(armed.current);
      chording.current = false;
      const to = CHORD[event.key];
      if (to === undefined) return;
      event.preventDefault();
      const pin = PINNED.find((one) => one.surface === to);
      go(pin?.id ?? `place:${to}`);
      return;
    }
    if (calls.open) return;
    const digit = PINNED.find((one) => one.digit === event.key);
    if (digit !== undefined && !event.shiftKey) {
      event.preventDefault();
      go(digit.id);
      return;
    }
    if (event.key === "[" || event.key === "]") {
      event.preventDefault();
      const at = ids.indexOf(place);
      go(ids[(at + (event.key === "]" ? 1 : -1) + ids.length) % ids.length]!);
      return;
    }
    if (event.key === "." || event.key === ",") {
      event.preventDefault();
      needs(event.key === "." ? 1 : -1);
      return;
    }
    if (event.key === "g") {
      event.preventDefault();
      chording.current = true;
      armed.current = window.setTimeout(() => (chording.current = false), 1500);
      return;
    }
    if (event.key === "w") {
      event.preventDefault();
      calls.openFront();
      return;
    }
    // `x` is Kill on a Job, so a Job tab closes with ⌘W and its mark alone.
    if (event.key === "x" && !isPinned(place) && !place.startsWith("job:")) {
      event.preventDefault();
      close(place);
      return;
    }
    // The merge line's real page has rows to read and no Merge: a ready pull request is merged from the title line.
    if (event.key === "m" && place === "merge") {
      const owner = readyPrs(ship).flatMap((pr) => ship.subjects.filter((one) => one.pr?.key === pr.key && one.mergeable))[0];
      if (owner !== undefined) {
        event.preventDefault();
        acts.merge(owner);
      }
      return;
    }
    if (!onCockpit) return;
    const subject = subjectOf(event);
    const at = subject === undefined ? -1 : order.indexOf(subject);
    const move = (by: number) => {
      const next = order[Math.min(order.length - 1, Math.max(0, at + by))];
      if (next === undefined) return;
      ui.select(next.key);
      askFocus(next.key);
    };
    switch (event.key) {
      case "j":
      case "ArrowDown":
        return event.preventDefault(), move(1);
      case "k":
      case "ArrowUp":
        return event.preventDefault(), move(-1);
      case "Escape":
        if (ui.sheet || ui.peeking !== undefined) return event.preventDefault(), ui.setSheet(false), ui.setPeeking(undefined);
        return;
      case "f": {
        const red = ship.mains.find((one) => one.state === "red" && !one.fixing);
        return red === undefined ? undefined : (event.preventDefault(), acts.fix(red));
      }
    }
    if (subject === undefined) return;
    switch (event.key) {
      case "Enter":
        if ((event.target as HTMLElement).tagName === "BUTTON") return;
        return event.preventDefault(), acts.open(subject);
      case "r":
        return subject.repliable ? (event.preventDefault(), reply(subject)) : undefined;
      case "p":
        return event.preventDefault(), ui.select(subject.key), ui.setPeeking(ui.peeking === subject.key ? undefined : subject.key);
      case "s":
        return subject.stoppable ? (event.preventDefault(), acts.stop(subject)) : undefined;
      case "m":
        return subject.mergeable ? (event.preventDefault(), acts.merge(subject)) : undefined;
    }
  });

  const active = views.find((one) => one.id === place);
  return (
    <>
      <div className="tabs-chrome">
        <TabBar tabs={views} active={place} onPick={go} onClose={close} />
        <TitleLine props={props} ship={ship} active={place} subject={active?.subject} titleOf={titleOf} onGo={go} acts={acts} jump={() => needs(1)} />
        {calls.tray}
        {/* Said to a screen reader and read by the walk: with no card open a key is heard by the window again. */}
        <span role="status" aria-label={calls.open ? "Call card open" : "Call card closed"} className="tabs-quiet" />
      </div>
      <div className="armada-cockpit proto-app-overlay" data-layer>
        {calls.card}
        <KeySheet ui={ui} more={KEYS_MORE} />
      </div>
    </>
  );
}

// ---- A page's scroll, kept for its tab -------------------------------------------------------------

/** A scroller named by its first class and its place among the others of that class: stable across a remount. */
function scrollerKey(element: HTMLElement): string | undefined {
  const first = element.classList[0];
  const mount = document.querySelector(".armada-screen__mounted");
  if (first === undefined || mount === null || !mount.contains(element)) return undefined;
  return `${first}#${[...mount.querySelectorAll(`.${CSS.escape(first)}`)].indexOf(element)}`;
}

function restoreScroll(id: string): void {
  const saved = scrollOf(id);
  const keys = Object.keys(saved);
  if (keys.length === 0) return;
  let tries = 0;
  const tick = () => {
    const mount = document.querySelector(".armada-screen__mounted");
    let waiting = false;
    for (const key of keys) {
      const [name, index] = key.split("#") as [string, string];
      const element = mount?.querySelectorAll<HTMLElement>(`.${CSS.escape(name)}`)[Number(index)];
      if (element === undefined || element.scrollHeight - element.clientHeight < saved[key]!) waiting = true;
      else element.scrollTop = saved[key]!;
    }
    if (waiting && (tries += 1) < 30 && tabsNow() !== undefined) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
}

// ---- The Cockpit tab -------------------------------------------------------------------------------

/** One Job or Session on one line: its mark, name, where it is, and the acts it carries, in place. */
function Row({ subject, selected, hot, onHot, ui, acts, onReply }: { subject: Subject; selected: boolean; hot: boolean; onHot: (number: number | undefined) => void; ui: ShipUi; acts: Acts; onReply: (subject: Subject) => void }) {
  const tone = toneOf(subject);
  const peek = ui.peeking === subject.key;
  return (
    <li
      className="tabs-row"
      data-subject={subject.key}
      data-tone={tone}
      data-unit={subject.pr?.key}
      data-selected={selected || undefined}
      data-hot={hot || undefined}
      aria-label={subject.title}
      tabIndex={0}
      onFocus={() => ui.select(subject.key)}
      onClick={() => ui.select(subject.key)}
      onPointerEnter={() => onHot(subject.pr?.number)}
      onPointerLeave={() => onHot(undefined)}
    >
      <MarkGlyph mark={{ ...subject.mark, tone }} />
      <span className="tabs-row__title">{subject.title}</span>
      <span className="tabs-row__repo">{subject.repo}</span>
      <span className="tabs-row__step">{subject.step === undefined ? null : <StepsBar step={subject.step} tone={tone} />}</span>
      <span className="tabs-row__pr">
        {subject.pr === undefined ? null : <PrChip pr={subject.pr} />}
        {subject.mergeable ? (
          <Button variant="secondary" size="sm" aria-label={`Merge #${subject.pr!.number}`} onClick={() => acts.merge(subject)}>
            Merge
          </Button>
        ) : null}
      </span>
      <span className="tabs-row__since">{subject.since ?? ""}</span>
      <span className="tabs-row__acts">
        {subject.repliable ? <IconAct label="Reply" keys="r" glyph={Reply} onPress={() => onReply(subject)} /> : null}
        <IconAct label="Peek" keys="p" glyph={Terminal} pressed={peek} onPress={() => ui.setPeeking(peek ? undefined : subject.key)} />
        {subject.stoppable ? <IconAct label="Stop" keys="s" glyph={Square} onPress={() => acts.stop(subject)} /> : null}
        <IconAct label="Open" keys="↵" glyph={ExternalLink} onPress={() => acts.open(subject)} />
      </span>
      {peek ? <PeekPop subject={subject} ui={ui} acts={acts} /> : null}
    </li>
  );
}

/**
 * The merge line, as the Cockpit's own horizon draws it: the dots, nearest main first, ending at main's light.
 * What is added is main's Fix beside the light, and the join to the strips above: the pull request a dot is and
 * the pull request a strip carries light each other up, by hover, focus or selection. The dots are the
 * Cockpit's, so what a dot is, and what its card says, is exactly what it is on main.
 */
const numberOf = (element: Element): number | undefined => {
  const said = /^#(\d+)/.exec(element.getAttribute("aria-label") ?? "");
  return said === null ? undefined : Number(said[1]);
};

function Foot({ props, ship, hot, onHot, acts, ui }: { props: CockpitProps; ship: Ship; hot: number | undefined; onHot: (number: number | undefined) => void; acts: Acts; ui: ShipUi }) {
  const sessions = useSessions();
  const foot = useRef<HTMLDivElement>(null);
  const main = ship.main;
  // A dot is drawn by the Cockpit and knows nothing of this page: how loud it is and which one is lit are said on it here.
  useEffect(() => {
    for (const dot of foot.current?.querySelectorAll<HTMLElement>(".armada-view__dot") ?? []) {
      const number = numberOf(dot);
      const owner = number === undefined ? undefined : ship.subjects.find((one) => one.kind !== "pr" && one.pr?.number === number);
      const tone = owner === undefined ? "calm" : toneOf(owner) !== "calm" ? toneOf(owner) : owner.mergeable ? "ask" : "calm";
      dot.dataset.tone = tone;
      if (number !== undefined && number === hot) dot.dataset.hot = "";
      else delete dot.dataset.hot;
    }
  });
  const named = (event: { target: EventTarget }) => (event.target instanceof Element ? numberOf(event.target.closest(".armada-view__dot") ?? event.target) : undefined);
  return (
    <div
      ref={foot}
      className="tabs-foot"
      data-main
      data-state={main?.state ?? "unknown"}
      data-fixing={main?.fixing || undefined}
      data-selected={ui.selected === "main" || undefined}
      onPointerOver={(event) => onHot(named(event))}
      onPointerOut={() => onHot(undefined)}
      onFocus={(event) => onHot(named(event))}
      onBlur={() => onHot(undefined)}
    >
      <Horizon state={props.state} sessions={sessions} hosts={props} />
      {main === undefined || main.state !== "red" ? null : (
        <span className="tabs-foot__main">
          <span className="tabs-main__check">{main.check}</span>
          {main.fixing ? (
            <Tooltip label="Fixing main">
              <span className="tabs-mark" data-tone="calm" role="img" aria-label="Fixing main">
                <Hammer size={16} aria-hidden />
              </span>
            </Tooltip>
          ) : (
            <FixAct main={main} acts={acts} />
          )}
        </span>
      )}
    </div>
  );
}

/** The Cockpit tab's page: the Jobs and Sessions by standing, one line each, and the merge line along the foot. */
export function TabsBody(props: CockpitProps) {
  const ship = useShip(props, false);
  const ui = useShipUi();
  const acts = useShipActs(props);
  const tabs = useTabs();
  const groups = groupedOf(ship);
  const order = groups.flatMap((one) => one.rows);
  // The pull request lit on both the strip and its dot: the one under the pointer, else the selected strip's.
  const [pointed, setPointed] = useState<number | undefined>(undefined);
  const current = order.find((one) => one.key === ui.selected) ?? (ui.selected === undefined ? order[0] : undefined);
  const reply = (subject: Subject) => {
    ui.select(subject.key);
    ui.setPeeking(subject.key);
    window.setTimeout(() => document.querySelector<HTMLElement>('.proto-peek__foot [role="textbox"], .proto-peek__foot textarea')?.focus(), 120);
  };
  // A key or the tab bar asked for something to be put under focus: here once the page is drawn.
  useEffect(() => {
    const key = tabs.focusing;
    if (key === undefined) return;
    const found = key === "main" ? document.querySelector<HTMLElement>(".tabs-foot__main button, [data-main]") : document.querySelector<HTMLElement>(key.startsWith("unit:") ? `[data-unit="${key.slice(5)}"]` : `[data-subject="${key}"]`);
    if (found === null) return;
    found.focus();
    found.scrollIntoView({ block: "nearest" });
    askFocus(undefined);
  }, [tabs.focusing, ship]);
  return (
    <div className="armada-cockpit proto tabs-cockpit" data-direction="tabs">
      <section className="tabs-page" aria-label="Cockpit">
        <div className="tabs-page__rows">
          {groups.map(({ group, rows }) => (
            <section key={group.id} className="tabs-group" data-group={group.id} aria-label={group.name}>
              <header className="tabs-band">{group.name}</header>
              <ul className="tabs-group__rows" aria-label={group.name}>
                {rows.map((one) => (
                  <Row key={one.key} subject={one} selected={current?.key === one.key} hot={hot !== undefined && one.pr?.number === hot} onHot={setPointed} ui={ui} acts={acts} onReply={reply} />
                ))}
              </ul>
            </section>
          ))}
        </div>
        <Foot props={props} ship={ship} hot={hot} onHot={setPointed} acts={acts} ui={ui} />
      </section>
    </div>
  );
}

// Autopilot's lane on the Cockpit, #2092: what Fleet holds of each repository's Autopilot, kept current
// by `autopilot.changed`, and the places it is drawn — the control in the header band, which opens the bar
// for the picked repository at the top of the Cockpit, above the filter band and outside the glass, on every
// filter; its marks and sidings on the Bays metro; and each set-aside pick as a tile in Active. One hook, so
// `Cockpit.tsx` adds a line where each is drawn and nothing else.

import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { Plane } from "lucide-react";
import { AutopilotBar, type AutopilotFields, type AutopilotPress, type AutopilotReading } from "@armada/components";
import type { Autopilot, AutopilotPick, IssueList, Outcome, RepositorySummary } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { AutopilotActed } from "../../../shared/api/autopilot";
import type { BridgeState } from "../../../shared/bridge";
import type { Item } from "../Dashboard";
import type { BaysAutopilot } from "./BaysView";
import { withAutopilot } from "./autopilot-metro";
import { cappedOf, capsOf, fieldsOf, isOn, localNow, nextUpOf, readingOf, settingsFrom, setAsideOf, stoppingOf } from "./autopilot";
import { AsideActs, AutopilotSidings, type SidingPress } from "./AutopilotSidings";
import "./autopilot.css";

/** A window whose Fleet serves no Autopilot (a fake without the slice) draws none. */
const served = () => typeof window.armada?.listAutopilots === "function";

/** Every repository's Autopilot this window has heard of, by Manifest, kept current by the event. */
function useAutopilots(manifestId: string | null): [ReadonlyMap<string, Autopilot>, (autopilot: Autopilot) => void] {
  const [held, setHeld] = useState<ReadonlyMap<string, Autopilot>>(new Map());
  const fold = useCallback((autopilot: Autopilot) => setHeld((was) => new Map(was).set(autopilot.manifest_id, autopilot)), []);
  useEffect(() => {
    if (!served()) return;
    return window.armada.onAutopilotChanged(fold);
  }, [fold]);
  useEffect(() => {
    if (!served()) return;
    let current = true;
    // One repository's, where one is picked: Fleet refuses a repository that never had one, which is off.
    if (manifestId !== null) void window.armada.getAutopilot(manifestId).then((read) => current && read.ok && fold(read.value));
    else void window.armada.listAutopilots().then((read) => current && read.ok && read.value.autopilots.forEach(fold));
    return () => {
      current = false;
    };
  }, [manifestId, fold]);
  return [held, fold];
}

/** The open issues of each repository with an Autopilot on, read again as its picks change. */
function useIssues(on: readonly Autopilot[]): ReadonlyMap<string, IssueList> {
  const [lists, setLists] = useState<ReadonlyMap<string, IssueList>>(new Map());
  const asked = on.map((one) => `${one.manifest_id}:${one.picks.length}`).join(" ");
  useEffect(() => {
    if (typeof window.armada?.listIssues !== "function") return;
    let current = true;
    for (const one of on) {
      void window.armada.listIssues(one.manifest_id, false).then((read) => current && read.ok && setLists((was) => new Map(was).set(one.manifest_id, read.value)));
    }
    return () => {
      current = false;
    };
  }, [asked]);
  return lists;
}

const NO_ASIDES: Item[] = [];

type Pressing = { manifest: string; press: AutopilotPress } | undefined;

/** The header band's control: how the picked repository's Autopilot stands, and whether its bar is open. */
export type AutopilotControl = {
  state: "off" | "running" | "paused";
  open: boolean;
  /** Opens the bar, or puts it away. Putting it away stops nothing. */
  toggle: () => void;
  /** The press with no repository picked opens the bar for the one about to be. */
  openNext: () => void;
};

export type AutopilotLane = {
  /** The picked repository's bar, drawn at the top of the Cockpit while the control has it open. Null otherwise. */
  bar: ReactNode;
  /** Absent where Fleet serves no Autopilot. */
  control: AutopilotControl | undefined;
  /** The metro's Autopilot parts, while one is on. */
  bays: BaysAutopilot | undefined;
  /** Each set-aside pick, as a tile in Active. */
  asides: Item[];
};

export function useAutopilotLane({ picked, state, now }: { picked: RepositorySummary | null; state: BridgeState; now: number }): AutopilotLane {
  const manifestId = picked?.manifest?.id ?? null;
  const [held, fold] = useAutopilots(manifestId);
  const nameOf = useCallback((manifest: string) => state.holds.repositories?.find((one) => one.manifest?.id === manifest)?.manifest?.repository ?? manifest, [state.holds.repositories]);
  const on = useMemo(() => [...held.values()].filter((one) => isOn(one) && (manifestId === null || one.manifest_id === manifestId)), [held, manifestId]);
  const issues = useIssues(on);

  const [pressing, setPressing] = useState<Pressing>();
  const [refused, setRefused] = useState<{ manifest: string; why: string }>();
  const [aside, setAside] = useState<SidingPress>();
  const [starting, setStarting] = useState(false);
  const [sentence, setSentence] = useState("");
  const [reading, setReading] = useState<AutopilotReading>();
  /** The repository whose Edit caps is open. */
  const [editing, setEditing] = useState<string>();
  const [open, setOpen] = useState(false);
  // With no repository picked there is no bar to have open.
  useEffect(() => {
    if (manifestId === null) setOpen(false);
  }, [manifestId]);

  /** Sends one act, folds Fleet's answer, and says a refusal where the press was. */
  const act = useCallback(
    async <T,>(manifest: string, press: AutopilotPress, send: () => Promise<AutopilotActed<T>>): Promise<T | undefined> => {
      setPressing({ manifest, press });
      setRefused(undefined);
      const answer = await send();
      setPressing(undefined);
      // Fleet's own words for a refusal: a held cap's names the cap.
      if (!answer.ok) return void setRefused({ manifest, why: refusalWords(answer.outcome as Outcome) });
      return answer.value;
    },
    [],
  );
  const acted = (manifest: string, press: AutopilotPress, send: () => Promise<AutopilotActed>) => void act(manifest, press, send).then((value) => value === undefined || fold(value));

  const read = useCallback(
    (manifest: string, words: string) => {
      setReading(undefined);
      void act(manifest, "read", () => window.armada.proposeAutopilot(manifest, words.trim(), localNow(Date.now()))).then((settings) => settings === undefined || setReading(readingOf(settings)));
    },
    [act],
  );

  // Another repository, another form.
  useEffect(() => {
    setStarting(false);
    setReading(undefined);
    setRefused(undefined);
    setEditing(undefined);
  }, [manifestId]);

  const start = (manifest: string, fields: AutopilotFields) => {
    const settings = settingsFrom(sentence, fields, Date.now());
    if (!settings.ok) return setRefused({ manifest, why: settings.why });
    void act(manifest, "start", () => window.armada.startAutopilot(manifest, settings.settings)).then((value) => {
      if (value === undefined) return;
      fold(value);
      setStarting(false);
      setReading(undefined);
    });
  };

  /** New settings for one that is on, in its state: raising the cap that paused it is what lets Resume through. */
  const save = (manifest: string, autopilot: Autopilot, fields: AutopilotFields) => {
    const settings = settingsFrom(autopilot.instruction, fields, Date.now());
    if (!settings.ok) return setRefused({ manifest, why: settings.why });
    void act(manifest, "save", () => window.armada.updateAutopilot(manifest, settings.settings)).then((value) => {
      if (value === undefined) return;
      fold(value);
      setEditing(undefined);
    });
  };

  const onAside = (manifest: string, pick: AutopilotPick, which: "run" | "dismiss") => {
    setAside({ pickId: pick.pick_id, act: which });
    const send = which === "run" ? window.armada.dispatchSetAside : window.armada.dismissSetAside;
    void act(manifest, which === "run" ? "start" : "stop", () => send(manifest, pick.pick_id)).then((value) => {
      setAside(undefined);
      if (value !== undefined) fold(value);
    });
  };

  const barOf = (manifest: string, autopilot: Autopilot | undefined) => {
    const live = isOn(autopilot);
    const mine = pressing?.manifest === manifest ? pressing.press : undefined;
    return (
      <AutopilotBar
        key={manifest}
        state={!live ? "off" : autopilot.state === "paused" ? "paused" : "running"}
        instruction={autopilot?.instruction}
        capped={autopilot === undefined ? undefined : cappedOf(autopilot)}
        fault={autopilot?.fault}
        caps={autopilot === undefined ? [] : capsOf(autopilot, now)}
        stopping={autopilot === undefined ? "" : stoppingOf(autopilot, state.jobs)}
        pending={aside === undefined ? mine : undefined}
        refused={refused?.manifest === manifest ? refused.why : undefined}
        onPause={() => acted(manifest, "pause", () => window.armada.pauseAutopilot(manifest))}
        onResume={() => acted(manifest, "resume", () => window.armada.resumeAutopilot(manifest))}
        onStop={() => acted(manifest, "stop", () => window.armada.stopAutopilot(manifest))}
        starting={starting}
        onStarting={(open) => {
          setStarting(open);
          if (!open) setReading(undefined);
          setRefused(undefined);
        }}
        sentence={sentence}
        onSentence={setSentence}
        reading={reading}
        onRead={() => read(manifest, sentence)}
        onStart={(fields) => start(manifest, fields)}
        editing={editing === manifest}
        onEditing={(open) => {
          setEditing(open ? manifest : undefined);
          setRefused(undefined);
        }}
        current={live ? fieldsOf(autopilot) : undefined}
        onSave={(fields) => live && save(manifest, autopilot, fields)}
      />
    );
  };

  // A tile that is neither a Job nor a Session, so the grid draws its own acts rather than Open.
  const asideItems = useMemo(() => on.flatMap((autopilot) =>
    setAsideOf(autopilot).map(
      (pick): Item => ({
        key: `aside:${autopilot.manifest_id}:${pick.pick_id}`,
        icon: Plane,
        kind: "Set aside by Autopilot",
        title: `#${pick.issue_number} ${pick.issue_title}`,
        fact: "Autopilot was unsure",
        hue: "issue",
        where: nameOf(autopilot.manifest_id),
        body: [["Why", pick.reason]],
        said: [pick.reason],
        acts: () => <AsideActs pick={pick} pending={aside} onRun={() => onAside(autopilot.manifest_id, pick, "run")} onDismiss={() => onAside(autopilot.manifest_id, pick, "dismiss")} />,
      }),
    ),
  ), [on, aside, nameOf]);

  const sidings = on.map((autopilot) => ({
    manifest: autopilot.manifest_id,
    name: manifestId === null ? nameOf(autopilot.manifest_id) : undefined,
    nextUp: nextUpOf(autopilot, issues.get(autopilot.manifest_id)),
    asides: setAsideOf(autopilot),
  }));
  const marks = useMemo(() => (metro: Parameters<BaysAutopilot["mark"]>[0]) => withAutopilot(metro, on), [on]);

  if (!served()) return { bar: null, control: undefined, bays: undefined, asides: NO_ASIDES };
  const mine = manifestId === null ? undefined : held.get(manifestId);
  const control: AutopilotControl = {
    state: !isOn(mine) ? "off" : mine.state === "paused" ? "paused" : "running",
    open: open && manifestId !== null,
    toggle: () => setOpen(!open),
    openNext: () => setOpen(true),
  };
  return {
    bar: open && manifestId !== null ? <div className="armada-autopilot-lane">{barOf(manifestId, mine)}</div> : null,
    control,
    bays: on.length === 0 ? undefined : { mark: marks, sidings: <AutopilotSidings sidings={sidings} pending={aside} onRun={(manifest, pick) => onAside(manifest, pick, "run")} onDismiss={(manifest, pick) => onAside(manifest, pick, "dismiss")} /> },
    asides: asideItems,
  };
}

// The two places the battlefield is drawn: the map, which is the stage when nothing is picked, and a Job's mission, the same plane
// cut down to the one Job in the canvas's place. Both read the field with the same hook, and both carry the same picked event and
// the same replay, so a row of Activity and a replay say the same thing wherever they are drawn. #2301.

import { useMemo, type ComponentProps } from "react";
import { Orbit } from "lucide-react";
import { taskGroupsOf } from "@armada/jobs/draft/group";

import type { Cockpit } from "../Cockpit";
import { Battlefield } from "./Battlefield";
import { missionOf, type SceneIn } from "./mission";
import type { DroneIn } from "./layout";
import type { Run } from "./panes";
import { FocusChip, ReplayBar, ReplayMark } from "./ReplayBar";
import { select, selOfPr, selOfSubject, startReplay, stopReplay } from "./store";
import { touchesOfFootprint } from "./touches";
import { useField } from "./useField";
import type { Ship, Subject } from "./ship";

type CockpitProps = ComponentProps<typeof Cockpit>;

/** The picked event's caption, the replay's marker and its bar: over the field, where it is drawn. */
function Overlays({ props, field }: { props: CockpitProps; field: ReturnType<typeof useField> }) {
  const { replay, picked, ordered } = field;
  // A replay can start only where the events Bridge holds reach: from an event in the window, and never earlier.
  const reaches = picked !== undefined && ordered.some((one) => one.id === picked.id);
  return (
    <>
      {replay === undefined ? null : <ReplayMark />}
      {replay !== undefined || picked === undefined ? null : <FocusChip event={picked} now={props.now} onReplay={reaches ? () => startReplay(Date.parse(picked.at)) : undefined} />}
      {replay === undefined ? null : <ReplayBar events={ordered} replay={replay} end={props.now} onLive={stopReplay} />}
    </>
  );
}

export function MapView({ props, ship, listed, cursor }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; cursor: string | undefined }) {
  const field = useField(props, ship, listed);
  const { replay, calls, repos, drones, pads, goals, crates, flightTrails, drawn, facts, clashes, focusKey } = field;
  return (
    <section className="deck-pane" aria-label="Flight map">
      <header className="deck-band deck-pane__head">
        <Orbit size={16} aria-hidden="true" />
        <h2 className="deck-pane__title">Map</h2>
      </header>
      <div className="deck-map">
        <Overlays props={props} field={field} />
        <Battlefield
          repos={repos}
          drones={drones}
          pads={pads}
          goals={goals}
          prs={crates}
          trails={flightTrails}
          plays={drawn}
          facts={facts}
          calls={calls}
          clashes={clashes}
          focus={replay === undefined ? focusKey : undefined}
          selected={cursor}
          onSelect={(key) => {
            const subject = ship.subjects.find((one) => one.key === key);
            if (subject !== undefined) select(selOfSubject(subject));
          }}
          onPad={(key) => select({ kind: "slot", key })}
          onGoal={(root) => {
            const pr = ship.prs.find((one) => one.root === root && one.stage !== "landed") ?? ship.prs.find((one) => one.root === root);
            if (pr !== undefined) select(selOfPr(pr));
          }}
          onPr={(key) => {
            const pr = ship.prs.find((one) => one.key === key);
            if (pr !== undefined) select(selOfPr(pr));
          }}
        />
      </div>
    </section>
  );
}

/**
 * A Job's mission: the battlefield's world cut down to the one Job. Its workflow is laid along a route of pins, the plots it touches
 * stand beside the route, its drone hovers over the step at work, and its Sessions and Drones stand near the steps they serve. The
 * same acts, the same feed and the same replay as the map; the field is read once, by the same hook, and cut to what is the Job's own.
 */
export function JobMission({ props, ship, listed, subject, run }: { props: CockpitProps; ship: Ship; listed: readonly Subject[]; subject: Subject; run: Run }) {
  const field = useField(props, ship, listed);
  const { replay, calls, drones, pads, goals, crates, flightTrails, drawn, facts, clashes, focusKey } = field;
  const job = subject.key;
  const begun = drones.some((one) => one.key === job);
  const mission = useMemo(() => (run.whole === null ? undefined : missionOf(run.whole, run.drones, begun)), [run.whole, run.drones, begun]);
  // The Job's drone, what it dispatched or was attached, and a small drone for each Drone working a task of a step. A stopped Job's
  // footprint places its drone where the edits were, which is all a running Job's observed bursts do not yet say.
  const footprint = run.whole?.footprint?.files;
  const cluster = useMemo<DroneIn[]>(() => {
    const own = drones
      .filter((one) => one.key === job || one.parent === job)
      .map((one) => ({ ...one, shown: true, ...(one.key === job && one.touches.length === 0 && footprint !== undefined ? { touches: touchesOfFootprint(footprint) } : {}) }));
    const tasks = run.whole === null ? [] : taskGroupsOf(run.whole).flatMap((group) => group.tasks.map((task) => ({ task, group: group.id })));
    const extra = run.drones
      .filter((one) => one.state === "running" && one.task !== undefined && one.id !== run.whole?.job.assigned_drone)
      .map<DroneIn>((one) => ({
        key: `drone:${one.id}`,
        kind: "session",
        title: tasks.find((t) => t.task.id === one.task)?.task.title ?? one.id,
        root: subject.root,
        said: "Working",
        hue: "calm",
        spin: true,
        hover: true,
        ping: false,
        stuck: false,
        flare: false,
        working: true,
        parent: job,
        touches: [],
        shown: true,
      }));
    return [...own, ...extra];
  }, [drones, job, run.drones, run.whole, subject.root, footprint]);
  const serves = useMemo(() => {
    const out = new Map<string, string>();
    for (const one of run.drones) {
      if (one.task === undefined) continue;
      const group = run.whole === null ? undefined : taskGroupsOf(run.whole).find((g) => g.tasks.some((t) => t.id === one.task));
      const pin = group === undefined ? undefined : `group:${group.id}`;
      out.set(`drone:${one.id}`, pin !== undefined && mission?.fans.size ? pin : `step:${one.step}`);
    }
    return out;
  }, [run.drones, run.whole, mission]);
  const scene = useMemo<Omit<SceneIn, "big" | "wide" | "clashes"> | undefined>(
    () => (mission === undefined ? undefined : { mission, repo: { root: subject.root, name: subject.repo }, drones: cluster, job, serves, pad: cluster.find((one) => one.key === job)?.slot }),
    [mission, cluster, job, serves, subject.root, subject.repo],
  );
  const keys = useMemo(() => new Set(cluster.map((one) => one.key)), [cluster]);
  const mine = useMemo(() => crates.filter((pr) => pr.owner === job), [crates, job]);
  const numbers = new Set(mine.map((pr) => pr.number));
  const bay = scene?.pad;
  const bays = useMemo(() => new Map([...pads].filter(([key]) => key === bay)), [pads, bay]);
  const trails = useMemo(() => new Map([...flightTrails].filter(([key]) => keys.has(key))), [flightTrails, keys]);
  const asking = useMemo(() => new Map([...calls].filter(([key]) => keys.has(key))), [calls, keys]);
  if (scene === undefined) return <div className="deck-job__reading" role="status" aria-busy aria-label="Reading the run" />;
  return (
    <>
      <Overlays props={props} field={field} />
      <Battlefield
        repos={[]}
        drones={cluster}
        pads={bays}
        goals={goals}
        prs={mine}
        trails={trails}
        plays={drawn.filter((play) => (play.key !== undefined && keys.has(play.key)) || (play.number !== undefined && numbers.has(play.number)))}
        facts={facts}
        calls={asking}
        clashes={clashes}
        focus={replay === undefined && focusKey !== undefined && keys.has(focusKey) ? focusKey : undefined}
        selected={undefined}
        scene={scene}
        onSelect={(key) => {
          const other = key === job ? undefined : ship.subjects.find((one) => one.key === key);
          if (other !== undefined) select(selOfSubject(other));
        }}
        onPad={(key) => select({ kind: "slot", key })}
        onGoal={() => {
          const pr = mine[0] === undefined ? undefined : ship.prs.find((one) => one.key === mine[0]!.key);
          if (pr !== undefined) select(selOfPr(pr));
        }}
        onPr={(key) => {
          const pr = ship.prs.find((one) => one.key === key);
          if (pr !== undefined) select(selOfPr(pr));
        }}
      />
    </>
  );
}

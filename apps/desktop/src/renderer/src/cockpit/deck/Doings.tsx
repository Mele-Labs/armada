// What happens on the ground: the plays the events drive (the cone of a Check turning on a drone, a ring where something was said, a ring
// from a plot that was touched) and what the working drones are doing over the plot they aim at (a scan, the laser's impact and its
// sparks, the ring a drone circles). At most `MOST_LOOPS` of the actions run as a loop; the rest show the same form still. Ported from
// proto/cockpit-flight-deck. #2301.

import { onPlane, px, vars } from "./bf-parts";
import type { Frame } from "./frame";
import { FLAT, type DroneIn, type Layout } from "./layout";
import type { Play } from "./plays";

export function Doings({ layout, frame, plays, droneBy, loops, focus }: { layout: Layout; frame: Frame; plays: readonly Play[]; droneBy: ReadonlyMap<string, DroneIn>; loops: ReadonlySet<string>; focus: string | undefined }) {
  const { floorAt, heights, aimOf, aimPoint, doers, orbit } = frame;
  return (
    <>
      {plays.map((play) => {
        const own = play.key === undefined ? undefined : layout.at.get(play.key);
        const drone = play.key === undefined ? undefined : droneBy.get(play.key);
        if (own === undefined || drone === undefined) return null;
        const at = onPlane(own);
        const place = { transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(floorAt(own) + 1.5)})` };
        if (play.kind === "sweep") {
          return (
            <div key={play.id} className="bf-at" style={place} aria-hidden="true">
              <i className="bf-cone" style={{ animationDelay: `${play.delay}ms` }} />
            </div>
          );
        }
        if (play.kind === "blip" || play.kind === "tracer") {
          return (
            <div key={play.id} className="bf-at" style={place} aria-hidden="true">
              <i className="bf-ripple" style={{ animationDelay: `${play.delay}ms` }} />
            </div>
          );
        }
        return null;
      })}
      {plays.flatMap((play) => {
        if ((play.kind !== "launch" && play.kind !== "sweep" && play.kind !== "tracer") || play.key === undefined) return [];
        const main = layout.over.get(play.key)?.[0]?.tile;
        if (main === undefined) return [];
        const at = onPlane({ x: main.cx, y: main.cy });
        return [
          <div key={`${play.id}:tile`} className="bf-at" style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px((heights.get(main.path) ?? 0) + 1.5)})` }} aria-hidden="true">
            <i className="bf-ripple" data-tile style={{ animationDelay: `${play.delay + (play.kind === "launch" ? 1000 : 300)}ms` }} />
          </div>,
        ];
      })}
      {doers.map((drone) => {
        const own = layout.at.get(drone.key)!;
        const tile = aimOf(drone)!;
        const loop = loops.has(drone.key);
        if (drone.doing === "run") {
          const centre = onPlane(own);
          return (
            <div key={`${drone.key}:do`} className="bf-at bf-do" data-doing="run" data-check={drone.check} data-loop={loop || undefined} data-focus={focus === drone.key || undefined} style={{ transform: `translate3d(${px(centre.x)}, ${px(centre.y)}, ${px(floorAt(own) + 1.5)})`, ...vars({ "--ring": px(orbit) }) }} aria-hidden="true">
              <i className="bf-do__orbit" />
              <i className="bf-do__radar" />
            </div>
          );
        }
        const at = onPlane(aimPoint(drone, tile));
        // A plot that is being written rises under the laser by up to the slab's rise: the impact rides on top of it.
        const z = (heights.get(tile.path) ?? 0) + (drone.doing === "write" ? 8.5 : 1.5);
        const wide = Math.min(tile.rect.w, 150) * 0.86;
        const deep = Math.min(tile.rect.h * FLAT, 100) * 0.86;
        return (
          <div key={`${drone.key}:do`} className="bf-at bf-do" data-doing={drone.doing} data-loop={loop || undefined} data-focus={focus === drone.key || undefined} style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(z)})`, ...vars({ "--w": px(wide), "--d": px(deep) }) }} aria-hidden="true">
            {drone.doing === "read" ? (
              <>
                <i className="bf-do__grid">
                  <i className="bf-do__scan" />
                </i>
                <i className="bf-do__pulse" />
              </>
            ) : (
              <>
                <i className="bf-do__impact" />
                {[0, 1, 2, 3].map((i) => (
                  <i key={i} className="bf-do__spark" style={vars({ "--i": i })} />
                ))}
              </>
            )}
          </div>
        );
      })}
    </>
  );
}

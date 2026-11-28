// To review, the screen Pocket opens on: everything every Job waits on, over every paired Mac, each answered in place.
// Groups stand as Fleet orders them: each sub project under its goal, then each feature, then the Jobs in neither. The
// switch under the bar goes between this and the Jobs list.

import { Box, Eye } from "lucide-react";

import { useFleet } from "./fleet";
import { MacRow } from "./JobList";
import { Notch } from "./Notch";
import { ReviewItem } from "./ReviewItem";
import { go, pairPath } from "./router";
import { useToReview, type Group } from "./reviewList";
import { Trouble } from "./ui";

/** To review and Jobs, the two places the switch goes between. */
export function Places({ here }: { here: "review" | "jobs" }) {
  const place = (to: "review" | "jobs", path: string, label: string, Glyph: typeof Eye) => (
    <button
      type="button"
      className="pk-places__place"
      aria-current={here === to ? "page" : undefined}
      data-here={here === to || undefined}
      onClick={() => here !== to && go(path)}
    >
      <Glyph size={14} aria-hidden="true" />
      {label}
    </button>
  );
  return (
    <nav className="pk-places" aria-label="To review or Jobs">
      {place("review", "/", "To review", Eye)}
      {place("jobs", "/jobs", "Jobs", Box)}
    </nav>
  );
}

function Head({ group }: { group: Group }) {
  const { head } = group;
  if (head.kind === "sub_project")
    return (
      <h2 className="pk-rv-group__head" data-kind="sub_project" title={head.name}>
        {head.goal}
      </h2>
    );
  if (head.kind === "feature")
    return (
      <h2 className="pk-rv-group__head" data-kind="feature">
        <span>{head.title}</span>
        <span className="pk-rv-group__area">{head.area_title}</span>
      </h2>
    );
  return (
    <h2 className="pk-rv-group__head" data-kind="neither">
      No feature
    </h2>
  );
}

export function ToReview() {
  const fleet = useFleet();
  const { groups, several, loaded, troubled } = useToReview();
  const down = fleet.macs.filter((mac) => !mac.paired);
  return (
    <section className="pk-screen" aria-label="To review">
      <Notch macs={fleet.macs} at={fleet.at} place="review" />
      <Places here="review" />
      <div className="pk-body pk-review">
        {down.length === 0 ? null : (
          <ul className="pk-jobs" aria-label="Macs">
            {down.map((mac) => (
              <MacRow
                key={mac.machine.origin}
                mac={mac}
                onPair={() => go(pairPath(mac.machine.origin))}
              />
            ))}
          </ul>
        )}
        {troubled.map(({ mac, says }) => (
          <Trouble
            key={mac.machine.origin}
            says={several ? `${mac.machine.name}: ${says}` : says}
          />
        ))}
        {!loaded && groups.length === 0 ? (
          <div className="pk-rv__wait pk-skeleton" aria-hidden="true" />
        ) : groups.length === 0 ? (
          <p className="pk-fact">Nothing waits on you.</p>
        ) : (
          groups.map((group) => (
            <section
              key={group.key}
              className="pk-rv-group"
              aria-label={
                group.head.kind === "sub_project"
                  ? group.head.goal
                  : group.head.kind === "feature"
                    ? group.head.title
                    : "No feature"
              }
            >
              <Head group={group} />
              <ul className="pk-rv-list">
                {group.entries.map(({ mac, item }) => (
                  <ReviewItem
                    key={`${mac.machine.origin} ${item.job.id} ${item.row.id}`}
                    mac={mac}
                    item={item}
                    several={several}
                  />
                ))}
              </ul>
            </section>
          ))
        )}
      </div>
    </section>
  );
}

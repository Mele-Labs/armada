// Settings → Layout: every tab, panel and rail row a layout can arrange, in the order it is drawn,
// each with whether it is drawn and the two moves. A change here is the owner's own choice, which
// outranks every layout mod; Reset to defaults puts the shipped layout back.

import { Button, LayoutRow } from "@armada/components";
import { LAYOUT, LAYOUT_REGIONS, layersOf, resolveLayout, useLayouts, type LayoutRegion } from "@armada/shell";

export function LayoutSettings() {
  const state = useLayouts();
  const { source } = state;
  const fromMods = layersOf(state).slice(0, -1);
  const own = state.own.regions;
  const changed = Object.keys(own).length > 0 || state.mods.some((mod) => mod.enabled);

  const move = (region: LayoutRegion, ids: readonly string[], at: number, by: number) => {
    const next = [...ids];
    const [one] = next.splice(at, 1);
    next.splice(at + by, 0, one!);
    source.setOwn(region, { ...own[region], order: next });
  };
  const show = (region: LayoutRegion, hidden: readonly string[], id: string, on: boolean) =>
    source.setOwn(region, { ...own[region], hidden: on ? hidden.filter((one) => one !== id) : [...hidden, id] });

  return (
    <div className="armada-layout-settings">
      {LAYOUT_REGIONS.map((region) => {
        const spec = LAYOUT[region];
        const { all } = resolveLayout(region, layersOf(state));
        const below = resolveLayout(region, fromMods).all;
        const ids = all.map((one) => one.id);
        const hidden = all.filter((one) => !one.visible).map((one) => one.id);
        return (
          <div key={region} className="armada-layout-settings__region" role="group" aria-label={spec.label}>
            <p className="armada-layout-settings__caption">{spec.label}</p>
            {all.map((one, at) => (
              <LayoutRow
                key={one.id}
                label={one.label}
                icon={one.icon}
                visible={one.visible}
                hideable={one.hideable}
                onVisible={(on) => show(region, hidden, one.id, on)}
                {...(one.hiddenBy === undefined || one.hiddenBy === "You" || below.find((was) => was.id === one.id)?.visible !== false ? {} : { hiddenBy: one.hiddenBy })}
                {...(spec.ordered && at > 0 ? { onUp: () => move(region, ids, at, -1) } : {})}
                {...(spec.ordered && at < all.length - 1 ? { onDown: () => move(region, ids, at, 1) } : {})}
              />
            ))}
          </div>
        );
      })}
      <div>
        <Button variant="secondary" disabled={!changed} onClick={() => source.toDefaults()}>
          Reset to defaults
        </Button>
      </div>
    </div>
  );
}

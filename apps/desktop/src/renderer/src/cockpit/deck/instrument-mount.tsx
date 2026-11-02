// The component of each instrument seated in a bay, kept mounted above the console, which gives way to a Job's keys and to the rack: closing a panel or opening a Job costs it no state.
// It renders into a node of its own (`slotOf`), which `ModFacet` stands in the panel while it is open. It reloads as a tab does (`useTabModule`), and a reload is another component.

import * as React from "react";
import { createPortal } from "react-dom";

import { useModList } from "../../mod-list";
import { Contained, useTabModule } from "../../job-panes/tab-host";
import { TAB_GLYPHS } from "../../job-panes/tab-glyph";
import { TAB_UI } from "../../job-panes/tab-ui";
import { keepBays, useBays } from "./ConsoleBays";
import { faceRegistered, forgetMount, mountLoaded, renderThrew, seatsFollowing, slotOf, useInstruments, type InstrumentHost } from "./instrument-mods";

/** Says the fault out of the boundary's render, where nothing may be set. */
function Threw({ name, load, why }: { name: string; load: number; why: string }) {
  React.useEffect(() => renderThrew(name, load, why), [name, load, why]);
  return null;
}

function InstrumentMount({ name }: { name: string }) {
  const { Component, load, problems, off } = useTabModule<InstrumentHost>(name, "instrument");
  const loaded = Component !== undefined;
  const refused = problems[0];
  // After the component's own effects, which is what lets a face registered at once stand through a reload.
  React.useEffect(() => mountLoaded(name, { load, loaded, refused, off }), [name, load, loaded, refused, off]);
  React.useEffect(() => () => forgetMount(name), [name]);
  const host = React.useMemo<InstrumentHost>(() => ({ React, armada: window.armada, ui: TAB_UI, glyphs: TAB_GLYPHS, face: (paint, options) => faceRegistered(name, load, paint, options?.moves === true) }), [name, load]);
  if (Component === undefined) return null;
  return createPortal(
    <Contained key={load} fault={(why) => <Threw name={name} load={load} why={why} />}>
      <Component host={host} />
    </Contained>,
    slotOf(name),
  );
}

/** One mount for each instrument seated, and the seats following what Fleet lists. */
export function InstrumentMounts() {
  const seated = useBays();
  const rows = useModList();
  const mods = useInstruments();
  React.useEffect(() => {
    const next = seatsFollowing(seated, rows);
    if (next !== seated) keepBays(next);
  }, [seated, rows]);
  const held = mods.filter((one) => seated.includes(one.seat));
  return (
    <>
      {held.map((one) => (
        <InstrumentMount key={one.name} name={one.name} />
      ))}
    </>
  );
}

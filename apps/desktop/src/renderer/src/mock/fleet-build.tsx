// The Fleet panel's build section, standing in for a Fleet that serves one: where the running build
// stands against main, which build it runs on, and the restart a choice or the button comes to.
// Seeded by scenario name; every other scenario draws no section.

import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type {
  FleetBuild,
  FleetBuildChoice,
  FleetBuildLeftOut,
  FleetBuildPosition,
  FleetBuildKept,
  FleetBuildPrevious,
  FleetBuildStage,
} from "@armada/components";

import { FleetBuildFrom } from "../fleet-build";
import { onTimePassing } from "./time-passes";

type Seed = {
  on: FleetBuildChoice;
  position: FleetBuildPosition;
  working: FleetBuildChoice | null;
  stage?: FleetBuildStage;
  failed?: string;
  leftOut?: FleetBuildLeftOut[];
  running?: FleetBuildKept;
  previous?: FleetBuildPrevious;
  reverting?: boolean;
  carried?: string[];
};

/** Commits in the preview that main lacks: the in-flight branches, merged. */
const PREVIEW_AHEAD = 4;
/** A restart holds each stage this long, then shows a Fleet on the other build. */
const STAGE_MS = 1_000;

/** The stages a restart onto each build passes through, in order. */
const STAGES: Record<FleetBuildChoice, readonly FleetBuildStage[]> = {
  main: [
    "fetching_main",
    "building_fleet",
    "building_bridge",
    "restarting_fleet",
    "reopening_bridge",
  ],
  preview: [
    "merging",
    "building_fleet",
    "building_bridge",
    "restarting_fleet",
    "reopening_bridge",
  ],
};

const SEEDS: Record<string, Seed> = {
  "fleet/build-aligned": {
    on: "main",
    position: {
      ahead: 0,
      behind: 0,
    },
    working: null,
  },
  "fleet/build-behind": {
    on: "main",
    position: {
      ahead: 0,
      behind: 3,
    },
    working: null,
  },
  "fleet/build-preview-ahead": {
    on: "preview",
    position: {
      ahead: PREVIEW_AHEAD,
      behind: 0,
    },
    working: null,
  },
  // A frame to look at: its restart is not timed. Each `later` step of its walk moves it on a stage,
  // and it holds at the last, so it does not finish on its own.
  "fleet/build-refreshing": {
    on: "preview",
    position: {
      ahead: PREVIEW_AHEAD,
      behind: 0,
    },
    working: "preview",
    stage: "merging",
  },
  "fleet/build-failed": {
    on: "main",
    position: {
      ahead: 0,
      behind: 3,
    },
    working: null,
    failed:
      "main cannot fast-forward to origin/main — it has diverged or a local change is in the way",
  },
  // The preview as the last refresh left it: two in-flight branches it could not take.
  "fleet/preview-left-out": {
    on: "preview",
    position: {
      ahead: PREVIEW_AHEAD,
      behind: 0,
    },
    working: null,
    leftOut: [
      {
        branch: "armada/projects-span-repos",
        why: "breaking migration projects.span_repositories is not on main",
      },
      {
        branch: "armada/issues-list-filters",
        why: "conflict: apps/desktop/src/renderer/src/issues-tab/list.tsx, packages/shell/src/Palette.tsx",
      },
    ],
  },
  // A kept build before this one, and two Jobs whose agents are working: `scenarios/fleet-build.ts`.
  "fleet/build-revert": {
    on: "preview",
    position: {
      ahead: PREVIEW_AHEAD,
      behind: 0,
    },
    working: null,
    running: {
      built: "5 Nov, 17:05",
      commit: "e4f5a6b",
    },
    previous: {
      built: "5 Nov, 16:20",
      commit: "a1b2c3d",
      bridgeReopens: true,
      takesOut: ["Fleet core and mods"],
      bringsBack: [],
    },
    carried: ["Operator board", "Store tests open a migrated copy"],
  },
  // Two branches left out, one with commits since: `scenarios/preview-offer.ts`.
  "preview-offer": {
    on: "preview",
    position: {
      ahead: PREVIEW_AHEAD,
      behind: 0,
    },
    working: null,
    leftOut: [
      {
        branch: "feat/billing-export",
        why: "merge conflict with feat/ledger",
        moved: true,
      },
      {
        branch: "feat/invoice-pdf",
        why: "breaking migration invoices.pdf_path is not on main",
      },
    ],
  },
};

const levelWith = (on: FleetBuildChoice): FleetBuildPosition =>
  on === "main"
    ? {
        ahead: 0,
        behind: 0,
      }
    : {
        ahead: PREVIEW_AHEAD,
        behind: 0,
      };

function Seeded({ seed, children }: { seed: Seed; children: ReactNode }) {
  const [now, setNow] = useState(seed);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  // A walk's `later` takes a restart to its next stage, and holds it at the last.
  useEffect(() => {
    onTimePassing(() =>
      setNow((was) => {
        if (was.working === null || was.stage === undefined) return was;
        const stages = STAGES[was.working];
        const next = stages[stages.indexOf(was.stage) + 1];
        return next === undefined
          ? was
          : {
              ...was,
              stage: next,
            };
      }),
    );
    return () => {
      onTimePassing(undefined);
      clearTimeout(timer.current);
    };
  }, []);
  const restart = (onto: FleetBuildChoice) => {
    const stages = STAGES[onto];
    const reach = (at: number) => {
      const stage = stages[at];
      if (stage === undefined) {
        setNow({
          on: onto,
          position: levelWith(onto),
          working: null,
        });
        return;
      }
      setNow(({ failed: _gone, ...was }) => ({
        ...was,
        working: onto,
        stage,
      }));
      timer.current = setTimeout(() => reach(at + 1), STAGE_MS);
    };
    reach(0);
  };
  // A revert holds two stages' time, then Fleet runs the build reverted to, with this one kept before it.
  const revert = () => {
    setNow((was) => ({
      ...was,
      reverting: true,
    }));
    timer.current = setTimeout(
      () =>
        setNow((was) => ({
          ...was,
          reverting: false,
          ...(was.previous === undefined || was.running === undefined
            ? {}
            : {
                running: {
                  built: was.previous.built,
                  commit: was.previous.commit,
                },
                previous: {
                  ...was.running,
                  bridgeReopens: was.previous.bridgeReopens,
                  takesOut: was.previous.bringsBack,
                  bringsBack: was.previous.takesOut,
                },
              }),
        })),
      STAGE_MS * 2,
    );
  };
  const build: FleetBuild = {
    on: now.on,
    position: now.position,
    working: now.working,
    ...(now.stage === undefined
      ? {}
      : {
          stage: now.stage,
        }),
    drones: [],
    ...(now.failed === undefined
      ? {}
      : {
          failed: now.failed,
        }),
    ...(now.leftOut === undefined
      ? {}
      : {
          leftOut: now.leftOut,
        }),
    ...(now.leftOut === undefined
      ? {}
      : {
          leftOut: now.leftOut,
        }),
    ...(now.previous === undefined
      ? {}
      : {
          previous: now.previous,
          ...(now.running === undefined
            ? {}
            : {
                running: now.running,
              }),
          reverting: now.reverting === true,
          carried: now.carried ?? [],
          onRevert: revert,
        }),
    onChoose: (on) => restart(on),
    onRestart: () => restart(now.on),
  };
  return <FleetBuildFrom value={build}>{children}</FleetBuildFrom>;
}

/** The app under this scenario's build, where it has one. */
export function MockFleetBuild({ scenario, children }: { scenario: string; children: ReactNode }) {
  const seed = SEEDS[scenario];
  return seed === undefined ? children : <Seeded seed={seed}>{children}</Seeded>;
}

// Revert to the build before, asked from the app menu or the palette. Everything it does runs in main,
// so it works with the page blank: a native confirm, then Fleet's own revert, and where Fleet is not
// there to ask, the undo the build before carries (`builds/previous/armada build revert`).

import type { FleetBuildReport, Outcome } from "@armada/protocol";

/** What the dialog is asked for: the part of `MessageBoxOptions` this flow uses. */
export type RevertBox = {
  type: "question" | "info" | "warning";
  message: string;
  detail: string;
  buttons: string[];
  defaultId: number;
  cancelId: number;
};

export type RevertAsking = {
  /** The latest published report. */
  report: () => FleetBuildReport | null;
  /** `FleetBuilds.revert`, or `undefined` where there is no connection to ask through. */
  revert: () => Promise<Outcome | undefined>;
  /** `dialog.showMessageBox`, parented to the main window where one is live. Answers the button pressed. */
  show: (box: RevertBox) => Promise<number>;
  /** The build before's own `armada`, or `null` where the support folder is unknown. */
  undo: () => string | null;
  exists: (file: string) => boolean;
  /** Start `file build revert` detached, with nothing attached to it. */
  run: (file: string, args: string[]) => void;
};

export const NO_BUILD_BEFORE = "No build before this one is kept";

/** True while a revert can be asked: a build before is kept and nothing is changing the build. */
export function canRevert(report: FleetBuildReport | null | undefined): boolean {
  return (
    report?.previous !== undefined &&
    report.reverting !== true &&
    report.restarting === undefined &&
    report.swapping !== true
  );
}

function confirmOf(previous: NonNullable<FleetBuildReport["previous"]>): RevertBox {
  const lines = [
    `Built ${previous.built_at}, commit ${previous.commit.slice(0, 8)}`,
  ];
  if (previous.takes_out !== undefined && previous.takes_out.length > 0)
    lines.push(`Takes out: ${previous.takes_out.join(", ")}`);
  if (previous.bridge_reopens === true) lines.push("Bridge reopens");
  return {
    type: "question",
    message: "Revert to the build before?",
    detail: lines.join("\n"),
    buttons: [
      "Revert",
      "Cancel",
    ],
    defaultId: 1,
    cancelId: 1,
  };
}

/** The reason line of a refusal or a Fleet that did not answer, as the Fleet panel words it. */
export function whyNot(outcome: Outcome): string {
  if (outcome.ok) return "";
  if (outcome.why === "refused") return outcome.error.message;
  if (outcome.why === "transport") return outcome.detail;
  return "Fleet did not answer";
}

export async function revertAsked(asking: RevertAsking): Promise<void> {
  const previous = asking.report()?.previous;
  const info = (message: string, detail: string) =>
    asking.show({
      type: "info",
      message,
      detail,
      buttons: [
        "OK",
      ],
      defaultId: 0,
      cancelId: 0,
    });
  if (previous === undefined) {
    await info(NO_BUILD_BEFORE, "");
    return;
  }
  if ((await asking.show(confirmOf(previous))) !== 0) return;
  const outcome = (await asking.revert()) ?? {
    ok: false as const,
    why: "not_connected" as const,
  };
  if (outcome.ok) return;
  if (outcome.why === "not_connected") {
    const file = asking.undo();
    if (file !== null && asking.exists(file))
      asking.run(file, [
        "build",
        "revert",
      ]);
    else await info("Fleet is not connected", "The build before has no undo to run");
    return;
  }
  await info("Revert did not start", whyNot(outcome));
}

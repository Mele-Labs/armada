// A whole screen, assembled, and the readings behind it.
//
// **Props in, callbacks out.** Nothing here opens a socket, reaches for a
// preload or knows what Electron is: a screen decides what the wire means and
// what a person may do about it, and the app it is mounted in does the asking.
// That is what lets a screen be rendered, storied and tested with no daemon.
//
// The host calls a screen needs arrive as arguments — `onReadDiff`,
// `onOpenArtifact`, `onNeedMaterial`, `onStage`, `onWant`. Each
// used to be a `window.armada` call written inline, which is precisely what
// held these files inside the app.

export * from "./Acts";
export * from "./Decide";
export * from "./DispatchJob";
export * from "./JobDetail";
export * from "./Log";
export { LandCheckLogSheet } from "./check-log-sheet";
export * from "./Overrule";
export * from "./RaiseCap";
export * from "./RaiseTurnCap";
export * from "./Redirect";
export * from "./Report";
export * from "./Reports";
export * from "./Lessons";
export type { AnswerLesson, LessonsTab, LessonsView, ReadLessons, ReadRetro } from "./retro";
export { lessonsTabNamed } from "./retro";
export * from "./Row";
export * from "./landed-words";
export * from "./Taken";
export * from "./freeze";
export * from "./Sheets";
// A repository's Studios, and one open on its whiteboard. #1287.
export * from "./studio";
export * from "./studio-frames";
export * from "./studio-reads";
export * from "./pending";
export * from "./title";
export * from "./board";
// The merge line `armada land` keeps, as Fleet serves it. Since 22.1.
export * from "./merge-line";
// On All repositories, the question a surface that needs one repository asks first.
export * from "./AskRepository";
export * from "./outputs";
export * from "./ManifestChecks";
export * from "./manifest-checks";
export * from "./checks";
export * from "./copy";
export * from "./editing";
export * from "./declared";
export * from "./detail-keys";
// Every question waiting on a person, from every repository, as Helm's dock draws them.
export * from "./dock-questions";
export * from "./refusal-words";
export * from "./outstanding";
export * from "./duration";
export * from "./facts";
export * from "./frozen";
export * from "./gates";
export * from "./held";
export * from "./keys";
export * from "./manifest-kit";
export * from "./verify";
export * from "./notes";
export * from "./opening";
export * from "./origin";
export * from "./phases";
export * from "./produced";
export * from "./proposal";
export * from "./reading";
export * from "./recovery";
export * from "./render";
export * from "./review";
export * from "./drawn-patch";
export * from "./run";
export * from "./StepActs";
export * from "./steering";
export * from "./story";
export * from "./waiting";
export * from "./work";
export * from "./open-studio";
// Setup and Locate are `@armada/setup`; their reads stay here, since main, the wire types and Manifest read them.
export type * from "./setup-reads";
export * from "./locate-reads";
// Overview's reads stay here, since Settings and the wire types read them; the surface is `@armada/overview`.
export type * from "./overview-reads";
// Workflow — the Job's run drawn as the workflow it froze, on the canvas or
// stacked, with the inspector beside it. #1539.
export * from "./tab-workflow";
export * from "./workflow-canvas";
export * from "./workflow-inspector";
export * from "./workflow-view";
// Plan — the same plan as a graph or as a list, the graph taken off Workflow.
export * from "./plan-canvas";
export * from "./plan-view";
// The wave a Job dispatched — which Jobs wait on which, and which are asking
// you something. #1544.
export * from "./tab-wave";
export * from "./wave";
export * from "./wave-plan";
export * from "./pausing";

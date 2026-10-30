// The document Flight 4 writes for the film, as a Job writes one: a free HTML page that links the kit. Mock only; the scenario
// `video-multitasking-boards` hands it to the reading layer through its `readArtifact`.

export const NOTES_TITLE = "Release notes for 2.4";
export const NOTES_PATH = "/Users/user/code/relay-docs/.armada/documents/release-notes-2-4.html";
/** The line the film selects to leave its note on. */
export const NOTES_FIRST = "Payments SDK v4 with the new webhooks";

export const NOTES_PAGE = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>${NOTES_TITLE}</title>
<link rel="stylesheet" href="armada-kit.css">
<style>
  body { max-inline-size: 46rem; }
  .kicker { margin: 0 0 var(--space-2); color: var(--fg-subtle); font-family: var(--font-mono); font-size: var(--text-2xs); line-height: var(--leading-2xs); letter-spacing: var(--tracking-caps); text-transform: uppercase; }
  .lede { font-size: var(--text-lg); line-height: var(--leading-lg); }
  h2 { padding-block-start: var(--space-3); border-block-start: var(--border-width) solid var(--border-subtle); }
</style>
</head>
<body>
<header>
  <p class="kicker">Relay 2.4</p>
  <h1>${NOTES_TITLE}</h1>
  <p class="lede">Relay 2.4 moves payments to the v4 SDK, adds a dark theme to Settings and makes the checkout tests reliable.</p>
</header>

<section>
  <h2>What is new</h2>
  <ul>
    <li>${NOTES_FIRST}</li>
    <li>Dark mode for Settings, following the system theme</li>
    <li>Release notes in the app, one page for each version</li>
  </ul>
</section>

<section>
  <h2>Fixes</h2>
  <ul>
    <li>The checkout test no longer fails at random when two orders are placed in one second</li>
    <li>The webhook handler registers once at start up</li>
  </ul>
</section>

<section>
  <h2>Upgrading</h2>
  <p>Webhooks registered with the v1 SDK stop working on 2.4. Register them again after the upgrade.</p>
  <ol>
    <li>Update the payments SDK to v4</li>
    <li>Register the webhooks</li>
    <li>Run the checkout tests</li>
  </ol>
</section>
</body>
</html>
`;

// Which Job's detail is open, as the page says it — for the annotation layer.
//
// **A note left with a Job's detail open is linked to that Job's retro**, by
// the id it carries as `openJobId`, and by nothing else (`docs/concepts/retro.md`,
// *The owner's annotations*). The layer is a root of its own beside the app's,
// so it reads the page rather than the app's state: Job detail stamps its id
// on its own root, and a note saved while that root is drawn carries it.
//
// **Job detail alone counts as open.** A retro read on the Lessons page is
// not that Job's detail, and a note left there names no Job.

/** The attribute Job detail's root carries its Job's id on. */
export const OPEN_JOB_ATTRIBUTE = "data-armada-open-job";

/** The id of the Job whose detail is drawn, or `undefined` where none is. */
export function openJobIn(doc: Document): string | undefined {
  const id = doc.querySelector(`[${OPEN_JOB_ATTRIBUTE}]`)?.getAttribute(OPEN_JOB_ATTRIBUTE);
  return id === null || id === undefined || id === "" ? undefined : id;
}

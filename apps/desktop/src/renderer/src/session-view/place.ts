// Where the owner stood in a Session when he left it for one it follows, so Back finds the view as he left it: how far the thread was
// scrolled up and which Plan groups were open. Held in the window and read off the page, because a Session's pane is drawn again when he
// comes back. A place is dropped when he opens anything not as a Back would return (`forgetPlace`).

type Place = { thread: number | undefined; groups: ReadonlySet<string> };

const left = new Map<string, Place>();

/** Within this many pixels of the end the thread follows new rows, so there is no place to keep. */
const NEAR_END = 40;

/** Notes where he is in Session `id`, from the page as it is now. */
export function leaving(id: string): void {
  const thread = document.querySelector<HTMLElement>('.deck-pane [role="region"][aria-label="Thread"]');
  const away = thread === null ? 0 : thread.scrollHeight - thread.clientHeight - thread.scrollTop;
  const groups = [...document.querySelectorAll<HTMLElement>(".sv-group[data-open]")].flatMap((one) => (one.dataset.group === undefined ? [] : [one.dataset.group]));
  left.set(id, { thread: thread === null || away <= NEAR_END ? undefined : thread.scrollTop, groups: new Set(groups) });
}

/** Whether Plan group `group` of Session `id` was open when he left. */
export const wasOpen = (id: string, group: string): boolean => left.get(id)?.groups.has(group) === true;

/** Puts the thread of Session `id` back where it was left, once its rows are drawn. */
export function restoreThread(id: string): void {
  const place = left.get(id);
  if (place?.thread === undefined) return;
  const top = place.thread;
  const put = () => {
    const thread = document.querySelector<HTMLElement>('.deck-pane [role="region"][aria-label="Thread"]');
    if (thread !== null) thread.scrollTop = top;
  };
  window.requestAnimationFrame(() => window.requestAnimationFrame(put));
}

export const forgetPlace = (): void => left.clear();

// What Bridge knows about a Job, kept where the annotation layer can read it. The layer has a root of its own beside the app's and
// must not import the app, and a Job's facts need its rows and Fleet's events, which only its pane holds. So the pane gives a function
// that reads them as they stand, and the layer asks for them by the Job's id when a note is sent.

const providers = new Map<string, () => readonly string[]>();

/** `facts` read as the Job stands each time it is asked. Returns the call that takes it back, which leaves a newer pane's for the same Job alone. */
export function provideJobFacts(id: string, facts: () => readonly string[]): () => void {
  providers.set(id, facts);
  return () => {
    if (providers.get(id) === facts) providers.delete(id);
  };
}

/** The Job's labelled fact lines, or `undefined` where no pane is giving them. */
export function jobFactsOf(id: string): readonly string[] | undefined {
  return providers.get(id)?.();
}

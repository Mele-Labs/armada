// A word the mock Fleet taught its Ticket lifecycle (`slices/lifecycles.ts`), told to the scenarios that serve Jobs in that repository, so a Job's own
// copy takes it and its lanes are drawn again as the real Fleet's re-read would. Each listener answers the Jobs whose copy took the word.

export type TaughtWord = {
  manifestId: string;
  word: string;
  state: string;
};

const hearing = new Set<(taught: TaughtWord) => readonly string[]>();

export const ticketWords = {
  listen(onTaught: (taught: TaughtWord) => readonly string[]): () => void {
    hearing.add(onTaught);
    return () => void hearing.delete(onTaught);
  },
  tell(taught: TaughtWord): string[] {
    return [
      ...hearing,
    ].flatMap((one) => one(taught));
  },
};

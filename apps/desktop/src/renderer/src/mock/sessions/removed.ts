// What Fleet tells an agent when the person removes an item it stated, as the mock words it.

export const removedSaid = (text: string, reason: string | undefined): string => {
  const why = reason?.trim() ?? "";
  return `Owner removed "${text}"${why === "" ? "" : `: ${why}`}`;
};

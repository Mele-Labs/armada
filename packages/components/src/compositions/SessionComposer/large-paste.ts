// A paste too big to send as words. It becomes a text file the agent is told the path of, so it reads as much of it as it needs
// rather than carrying all of it in every turn after.

/** Past this many tokens a paste is a file. A token is estimated at four characters, which a model's own count would only refine. */
export const LARGE_PASTE_TOKENS = 8_000;

/** What the words cost the agent, roughly: four characters a token. */
export const tokensEstimate = (text: string): number => Math.ceil(text.length / 4);

export const isLargePaste = (text: string): boolean => tokensEstimate(text) > LARGE_PASTE_TOKENS;

/** The words as a file, named by how many have been kept before: `pasted-snippet-1.txt`. */
export const snippetOf = (text: string, number: number): File => new File([text], `pasted-snippet-${number}.txt`, { type: "text/plain" });

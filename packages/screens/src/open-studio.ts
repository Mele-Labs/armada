// Open the Studio a Job came off. Its own file because Studios and Jobs both
// take it as a prop, and neither should import the other's file for a type.

/** Open the Studio a job came off, landing on the job's own node. */
export type OpenStudioFrom = (studioId: string, nodeId: string) => void;

import { describe, expect, it } from "vitest";

import { projectParent } from "./project-parent";

describe("the folder a new project is made in", () => {
  it("makes ~/code when it is missing, and answers it under home", async () => {
    const made: string[] = [];
    expect(await projectParent("~/code", "/Users/user", async (path) => void made.push(path))).toBe("/Users/user/code");
    expect(made).toEqual(["/Users/user/code"]);
  });

  it("makes no other folder, because every other one was chosen or read from where projects are", async () => {
    const made: string[] = [];
    const make = async (path: string) => void made.push(path);
    expect(await projectParent("/Users/user/Development", "/Users/user", make)).toBe("/Users/user/Development");
    expect(await projectParent("~/Development", "/Users/user", make)).toBe("/Users/user/Development");
    expect(made).toEqual([]);
  });
});

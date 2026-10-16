// The screens `App` mounts that draw an age, each reading the window's clock itself.

import { JobDetail } from "@armada/jobs";
import { Worktrees } from "@armada/cleanup";
import { Manifest } from "@armada/manifest";
import { Setup } from "@armada/setup";
import { clocked } from "./clocked";
import { DoctorSurface } from "./doctor/Doctor";
import { StudiosSurface } from "./StudiosSurface";

export const JobDetailNow = clocked(JobDetail);
export const WorktreesNow = clocked(Worktrees);
export const ManifestNow = clocked(Manifest);
export const SetupNow = clocked(Setup);
export const StudiosSurfaceNow = clocked(StudiosSurface);
export const DoctorSurfaceNow = clocked(DoctorSurface);

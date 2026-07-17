import type { SigningStatusDto } from "./types";

export function pathSatisfied(
  status: SigningStatusDto | null,
  pathId: string,
): boolean {
  if (!status?.paths.length) return false;
  const focus =
    status.paths.find((p) => p.path.id === pathId) ??
    status.paths.find((p) => p.satisfied) ??
    null;
  return focus?.satisfied ?? false;
}

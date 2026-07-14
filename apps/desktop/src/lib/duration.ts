/** Mirror of crates/policy-engine/src/timelock.rs — keep in sync. */
export const BLOCKS_PER_DAY = 144;
export const BLOCKS_PER_WEEK = 1_008;
export const BLOCKS_PER_YEAR = 52_560;

export type TimelockUnit = "d" | "w" | "y" | "b";

export function durationToBlocks(amount: number, unit: TimelockUnit): number {
  if (!Number.isFinite(amount) || amount < 0) return 0;
  const n = Math.floor(amount);
  switch (unit) {
    case "d":
      return n * BLOCKS_PER_DAY;
    case "w":
      return n * BLOCKS_PER_WEEK;
    case "y":
      return n * BLOCKS_PER_YEAR;
    case "b":
      return n;
  }
}

export function formatDuration(amount: number, unit: TimelockUnit): string {
  const n = Math.floor(amount);
  return unit === "b" ? `${n}b` : `${n}${unit}`;
}

export function parseDurationToBlocks(after: string): number | null {
  const input = after.trim();
  if (!input) return null;
  const match = /^(\d+)([ydwb])?$/i.exec(input);
  if (!match) return null;
  const amount = Number(match[1]);
  const unit = (match[2]?.toLowerCase() ?? "b") as TimelockUnit;
  if (!["d", "w", "y", "b"].includes(unit)) return null;
  return durationToBlocks(amount, unit);
}

export function formatTimelockLabel(after: string): string {
  const blocks = parseDurationToBlocks(after);
  if (blocks == null) return after;
  return `${after} (${blocks.toLocaleString()} blocks)`;
}

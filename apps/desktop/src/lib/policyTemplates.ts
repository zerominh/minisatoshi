import type { FallbackPolicy, KeyConfig, KeyRole, PolicyConfig } from "./types";
import { formatDuration, type TimelockUnit } from "./duration";

export type TemplateId =
  | "abc"
  | "two_of_three"
  | "inheritance"
  | "dead_mans_switch"
  | "multi_manager"
  | "custom";

export interface PolicyTemplate {
  id: TemplateId;
  label: string;
  description: string;
  /** Suggested key slots (id + role). User can add more for multi_manager / custom. */
  defaultKeys: { id: string; role: KeyRole; label: string }[];
  defaultPrimary: string;
  /** Suggested single inheritance/DMS fallback; empty = none. */
  defaultFallback?: { amount: number; unit: TimelockUnit; allow: string };
}

export const POLICY_TEMPLATES: PolicyTemplate[] = [
  {
    id: "abc",
    label: "ABC wallet",
    description: "(A∧B)∨(A∧C) now; after delay, investor A alone.",
    defaultKeys: [
      { id: "A", role: "investor", label: "Investor" },
      { id: "B", role: "manager", label: "Manager" },
      { id: "C", role: "recovery", label: "Recovery" },
    ],
    defaultPrimary: "(A && B) || (A && C)",
    defaultFallback: { amount: 4, unit: "y", allow: "A" },
  },
  {
    id: "two_of_three",
    label: "2-of-3 multisig",
    description: "Any two of A, B, C. Optional inheritance fallback.",
    defaultKeys: [
      { id: "A", role: "investor", label: "Key A" },
      { id: "B", role: "manager", label: "Key B" },
      { id: "C", role: "recovery", label: "Key C" },
    ],
    defaultPrimary: "(A && B) || (A && C) || (B && C)",
  },
  {
    id: "inheritance",
    label: "Inheritance",
    description: "A and B spend now; after delay, A alone.",
    defaultKeys: [
      { id: "A", role: "investor", label: "Investor / heir" },
      { id: "B", role: "manager", label: "Co-signer" },
    ],
    defaultPrimary: "A && B",
    defaultFallback: { amount: 4, unit: "y", allow: "A" },
  },
  {
    id: "dead_mans_switch",
    label: "Dead man's switch",
    description: "A spends freely; if inactive past delay, B can sweep.",
    defaultKeys: [
      { id: "A", role: "investor", label: "Primary" },
      { id: "B", role: "recovery", label: "Recovery" },
    ],
    defaultPrimary: "A",
    defaultFallback: { amount: 1, unit: "y", allow: "B" },
  },
  {
    id: "multi_manager",
    label: "Multi-manager",
    description: "Investor A with any one of several managers — add extra keys as needed.",
    defaultKeys: [
      { id: "A", role: "investor", label: "Investor" },
      { id: "B", role: "manager", label: "Manager 1" },
      { id: "C", role: "manager", label: "Manager 2" },
    ],
    defaultPrimary: "(A && B) || (A && C)",
  },
  {
    id: "custom",
    label: "Custom expression",
    description: "Edit keys, primary &&/|| expression, and recovery paths freely.",
    defaultKeys: [
      { id: "A", role: "investor", label: "Key A" },
      { id: "B", role: "manager", label: "Key B" },
    ],
    defaultPrimary: "A && B",
  },
];

export function emptyKey(id: string, role: KeyRole): KeyConfig {
  return { id, role, xpub: "", fingerprint: "", origin_path: "" };
}

export function keysFromTemplate(template: PolicyTemplate): KeyConfig[] {
  return template.defaultKeys.map((slot) => emptyKey(slot.id, slot.role));
}

/** `(A && B) || (A && C) || …` for investor A + remaining key ids. */
export function multiManagerPrimary(keyIds: string[]): string {
  const managers = keyIds.filter((id) => id !== "A");
  if (managers.length === 0) return "A";
  return managers.map((id) => `(A && ${id})`).join(" || ");
}

export function nextKeyId(existing: KeyConfig[]): string {
  const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  for (const ch of letters) {
    if (!existing.some((k) => k.id === ch)) return ch;
  }
  return `K${existing.length + 1}`;
}

export interface RecoveryPathDraft {
  amount: number;
  unit: TimelockUnit;
  allow: string;
}

export function draftToFallback(path: RecoveryPathDraft): FallbackPolicy {
  return {
    after: formatDuration(path.amount, path.unit),
    allow: path.allow.trim(),
  };
}

export function buildPolicyConfig(args: {
  network: PolicyConfig["network"];
  keys: KeyConfig[];
  primary: string;
  recoveryPaths: RecoveryPathDraft[];
}): PolicyConfig {
  return {
    version: 1,
    network: args.network,
    script_type: "taproot",
    keys: args.keys.map((k) => ({
      ...k,
      origin_path: k.origin_path || undefined,
    })),
    policy: {
      primary: args.primary.trim(),
      fallback: null,
      fallbacks: args.recoveryPaths
        .filter((p) => p.allow.trim() && p.amount >= 1)
        .map(draftToFallback),
    },
  };
}

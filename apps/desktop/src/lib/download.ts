import { invoke } from "@tauri-apps/api/core";

export type OpenTextFileResult = {
  path: string;
  contents: string;
};

export type TextFileFilter = {
  filterName?: string;
  filterExtensions?: string[];
};

/**
 * Native Save As dialog → write UTF-8 text.
 * Returns the absolute path, or `null` if the user cancelled.
 */
export async function saveTextFileWithDialog(
  defaultFilename: string,
  contents: string,
  filter?: TextFileFilter,
): Promise<string | null> {
  return invoke<string | null>("save_text_file", {
    defaultFilename,
    contents,
    filterName: filter?.filterName ?? null,
    filterExtensions: filter?.filterExtensions ?? null,
  });
}

/** Native Open dialog → read UTF-8 text. Returns `null` if cancelled. */
export async function openTextFileWithDialog(
  filter?: TextFileFilter,
): Promise<OpenTextFileResult | null> {
  return invoke<OpenTextFileResult | null>("open_text_file", {
    filterName: filter?.filterName ?? null,
    filterExtensions: filter?.filterExtensions ?? null,
  });
}

const PSBT_FILTER: TextFileFilter = {
  filterName: "PSBT",
  filterExtensions: ["psbt", "txt"],
};

export async function savePsbtFileWithDialog(
  defaultFilename: string,
  base64: string,
): Promise<string | null> {
  const body = base64.trim().endsWith("\n") ? base64.trim() : `${base64.trim()}\n`;
  return saveTextFileWithDialog(defaultFilename, body, PSBT_FILTER);
}

export async function openPsbtFileWithDialog(): Promise<OpenTextFileResult | null> {
  return openTextFileWithDialog(PSBT_FILTER);
}

export function sanitizedFilename(name: string, fallback = "wallet"): string {
  const cleaned = name
    .trim()
    .replace(/[^\w\-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return cleaned || fallback;
}

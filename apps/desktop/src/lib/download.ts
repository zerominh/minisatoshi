import { invoke } from "@tauri-apps/api/core";

/**
 * Native Save As dialog → write UTF-8 text.
 * Returns the absolute path, or `null` if the user cancelled.
 */
export async function saveTextFileWithDialog(
  defaultFilename: string,
  contents: string,
): Promise<string | null> {
  return invoke<string | null>("save_text_file", {
    defaultFilename,
    contents,
  });
}

export function sanitizedFilename(name: string, fallback = "vault"): string {
  const cleaned = name
    .trim()
    .replace(/[^\w\-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return cleaned || fallback;
}

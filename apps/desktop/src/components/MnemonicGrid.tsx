import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import jsQR from "jsqr";
import { BIP39_ENGLISH } from "../lib/seedQr";
import { parseSeedQrPayload } from "../lib/seedQr";

export type WordCount = 12 | 24;

type Props = {
  wordCount: WordCount;
  onWordCountChange: (n: WordCount) => void;
  words: string[];
  onWordsChange: (words: string[]) => void;
  disabled?: boolean;
};

function emptyWords(n: WordCount): string[] {
  return Array.from({ length: n }, () => "");
}

function parseClipboardToWords(text: string, wordCount: WordCount): string[] | null {
  try {
    const { mnemonic } = parseSeedQrPayload(text);
    const parts = mnemonic.split(/\s+/);
    if (parts.length !== 12 && parts.length !== 24) return null;
    return padWords(parts, wordCount);
  } catch {
    const parts = text
      .trim()
      .toLowerCase()
      .split(/[\s,]+/)
      .filter(Boolean);
    if (parts.length === 12 || parts.length === 24) {
      return padWords(parts, wordCount);
    }
    return null;
  }
}

function padWords(parts: string[], wordCount: WordCount): string[] {
  const n = parts.length === 12 || parts.length === 24 ? parts.length : wordCount;
  const out = emptyWords(n as WordCount);
  for (let i = 0; i < Math.min(parts.length, out.length); i++) {
    out[i] = parts[i];
  }
  return out;
}

export function MnemonicGrid({
  wordCount,
  onWordCountChange,
  words,
  onWordsChange,
  disabled,
}: Props) {
  const inputsRef = useRef<Array<HTMLInputElement | null>>([]);
  const [scanError, setScanError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const rafRef = useRef<number | null>(null);
  const fileRef = useRef<HTMLInputElement | null>(null);

  const columns = useMemo(() => {
    // Sparrow layout: column-major — col0 = 1..8, col1 = 9..16, col2 = 17..24
    const perCol = wordCount / 3;
    const sparrow: number[][] = [[], [], []];
    for (let c = 0; c < 3; c++) {
      for (let r = 0; r < perCol; r++) {
        sparrow[c].push(c * perCol + r);
      }
    }
    return sparrow;
  }, [wordCount]);

  useEffect(() => {
    if (words.length !== wordCount) {
      onWordsChange(emptyWords(wordCount));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only resize when count changes
  }, [wordCount]);

  function setWordAt(index: number, value: string) {
    const next = words.slice();
    while (next.length < wordCount) next.push("");
    const cleaned = value.toLowerCase().replace(/[^a-z]/g, "");
    next[index] = cleaned;
    onWordsChange(next.slice(0, wordCount));
  }

  function applyMnemonic(mnemonic: string) {
    const parts = mnemonic.trim().toLowerCase().split(/\s+/);
    const n: WordCount = parts.length === 12 ? 12 : 24;
    onWordCountChange(n);
    const next = emptyWords(n);
    for (let i = 0; i < Math.min(parts.length, n); i++) {
      next[i] = parts[i];
    }
    onWordsChange(next);
    setScanError(null);
  }

  function stopCamera() {
    if (rafRef.current != null) {
      cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    streamRef.current?.getTracks().forEach((t) => t.stop());
    streamRef.current = null;
    setScanning(false);
  }

  useEffect(() => () => stopCamera(), []);

  const tickScan = useCallback(() => {
    const video = videoRef.current;
    if (!video || video.readyState < 2) {
      rafRef.current = requestAnimationFrame(tickScan);
      return;
    }
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.drawImage(video, 0, 0);
    const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const code = jsQR(image.data, image.width, image.height, {
      inversionAttempts: "dontInvert",
    });
    if (code) {
      try {
        const result = parseSeedQrPayload(code.data, code.binaryData);
        applyMnemonic(result.mnemonic);
        stopCamera();
        return;
      } catch (err) {
        setScanError(err instanceof Error ? err.message : String(err));
      }
    }
    rafRef.current = requestAnimationFrame(tickScan);
  }, [wordCount]);

  async function startCamera() {
    setScanError(null);
    stopCamera();
    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: "environment" },
        audio: false,
      });
      streamRef.current = stream;
      setScanning(true);
      requestAnimationFrame(() => {
        const video = videoRef.current;
        if (!video) return;
        video.srcObject = stream;
        void video.play().then(() => {
          rafRef.current = requestAnimationFrame(tickScan);
        });
      });
    } catch (err) {
      setScanError(
        err instanceof Error
          ? `Camera unavailable: ${err.message}. Use “Scan image” instead.`
          : "Camera unavailable — use Scan image",
      );
      setScanning(false);
    }
  }

  async function onPickImage(file: File | null) {
    if (!file) return;
    setScanError(null);
    try {
      const bitmap = await createImageBitmap(file);
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const ctx = canvas.getContext("2d");
      if (!ctx) throw new Error("canvas unsupported");
      ctx.drawImage(bitmap, 0, 0);
      const image = ctx.getImageData(0, 0, canvas.width, canvas.height);
      const code = jsQR(image.data, image.width, image.height, {
        inversionAttempts: "attemptBoth",
      });
      if (!code) {
        throw new Error("No QR code found in image");
      }
      const result = parseSeedQrPayload(code.data, code.binaryData);
      applyMnemonic(result.mnemonic);
    } catch (err) {
      setScanError(err instanceof Error ? err.message : String(err));
    }
  }

  function onPasteField(index: number, text: string) {
    const multi = parseClipboardToWords(text, wordCount);
    if (multi) {
      onWordCountChange(multi.length === 12 ? 12 : 24);
      onWordsChange(multi);
      return true;
    }
    setWordAt(index, text);
    return false;
  }

  return (
    <div className="mnemonic-panel">
      <div className="mnemonic-toolbar">
        <div className="mnemonic-title">
          <strong>Mnemonic Words (BIP39)</strong>
          <span className="muted">
            {wordCount} words · English wordlist
          </span>
        </div>
        <div className="row-actions">
          <select
            aria-label="Word count"
            value={wordCount}
            disabled={disabled}
            onChange={(e) => onWordCountChange(Number(e.target.value) as WordCount)}
          >
            <option value={12}>12 words</option>
            <option value={24}>24 words</option>
          </select>
          <button
            type="button"
            className="secondary"
            disabled={disabled}
            onClick={() => void startCamera()}
          >
            Scan SeedQR
          </button>
          <button
            type="button"
            className="secondary"
            disabled={disabled}
            onClick={() => fileRef.current?.click()}
          >
            Scan image
          </button>
          <input
            ref={fileRef}
            type="file"
            accept="image/*"
            hidden
            onChange={(e) => void onPickImage(e.target.files?.[0] ?? null)}
          />
        </div>
      </div>

      {scanning ? (
        <div className="seedqr-camera">
          <video ref={videoRef} muted playsInline />
          <button type="button" className="secondary" onClick={stopCamera}>
            Stop camera
          </button>
        </div>
      ) : null}

      {scanError ? <p className="error compact">{scanError}</p> : null}

      <div className="mnemonic-grid" data-cols={3}>
        {columns.map((col, colIdx) => (
          <div key={colIdx} className="mnemonic-col">
            {col.map((index) => (
              <label key={index} className="mnemonic-cell">
                <span className="mnemonic-index">{index + 1}</span>
                <input
                  ref={(el) => {
                    inputsRef.current[index] = el;
                  }}
                  value={words[index] ?? ""}
                  disabled={disabled}
                  autoComplete="off"
                  autoCorrect="off"
                  autoCapitalize="off"
                  spellCheck={false}
                  list={`bip39-${index}`}
                  onChange={(e) => setWordAt(index, e.target.value)}
                  onPaste={(e) => {
                    const text = e.clipboardData.getData("text");
                    if (text.includes(" ") || text.includes(",") || /^\d{48,}$/.test(text.trim())) {
                      e.preventDefault();
                      onPasteField(index, text);
                    }
                  }}
                  onKeyDown={(e) => {
                    if (e.key === " " || e.key === "Enter" || e.key === "Tab") {
                      if ((words[index] ?? "").length > 0 && index < wordCount - 1) {
                        if (e.key !== "Tab") e.preventDefault();
                        inputsRef.current[index + 1]?.focus();
                      }
                    }
                  }}
                />
                <datalist id={`bip39-${index}`}>
                  {(words[index] ?? "").length >= 2
                    ? BIP39_ENGLISH.filter((w: string) =>
                        w.startsWith((words[index] ?? "").toLowerCase()),
                      )
                        .slice(0, 8)
                        .map((w: string) => (
                          <option key={w} value={w} />
                        ))
                    : null}
                </datalist>
              </label>
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

export function wordsToMnemonic(words: string[]): string {
  return words.map((w) => w.trim().toLowerCase()).filter(Boolean).join(" ");
}

export function mnemonicIsComplete(words: string[], wordCount: WordCount): boolean {
  return (
    words.length === wordCount &&
    words.every((w) => w.trim().length > 0) &&
    words.every((w) => BIP39_ENGLISH.includes(w.trim().toLowerCase()))
  );
}

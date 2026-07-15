import QRCode from "qrcode";
import { useEffect, useState } from "react";
import { Link, useLocation, useParams } from "react-router-dom";
import {
  exportBsms,
  exportWalletBackup,
  formatError,
  getWallet,
  openHotWallet,
} from "../lib/api";
import { saveTextFileWithDialog, sanitizedFilename } from "../lib/download";
import { splitDescriptorQrChunks } from "../lib/qrChunks";
import { copyText, getHwFingerprint } from "../lib/settings";
import type { WalletDto } from "../lib/types";
import { hasRememberedSigningDevice } from "../lib/watchOnly";

export function ShareWalletPage() {
  const { id = "" } = useParams();
  const location = useLocation();
  const isHot = location.pathname.startsWith("/hot-wallets/");
  const [walletId, setWalletId] = useState("");
  const [wallet, setWallet] = useState<WalletDto | null>(null);
  const [chunks, setChunks] = useState<string[]>([]);
  const [chunkIndex, setChunkIndex] = useState(0);
  const [qr, setQr] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setWallet(null);
    setError(null);
    void (async () => {
      try {
        const resolved = isHot ? (await openHotWallet(id)).id : id;
        if (cancelled) return;
        setWalletId(resolved);
        const w = await getWallet(resolved);
        if (cancelled) return;
        setWallet(w);
        setChunks(splitDescriptorQrChunks(w.descriptor));
        setChunkIndex(0);
      } catch (err) {
        if (!cancelled) setError(formatError(err));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [id, isHot]);

  useEffect(() => {
    const payload = chunks[chunkIndex];
    if (!payload) {
      setQr(null);
      return;
    }
    void QRCode.toDataURL(payload, {
      margin: 1,
      width: 280,
      errorCorrectionLevel: "M",
    })
      .then(setQr)
      .catch((err) => setError(formatError(err)));
  }, [chunks, chunkIndex]);

  async function onCopyDescriptor() {
    if (!wallet) return;
    await copyText(wallet.descriptor);
    setMessage("Copied descriptor");
  }

  async function onSaveDescriptor() {
    if (!wallet) return;
    setError(null);
    try {
      const path = await saveTextFileWithDialog(
        `${sanitizedFilename(wallet.name)}-descriptor.txt`,
        `${wallet.descriptor}\n`,
      );
      if (path) setMessage(`Descriptor saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onSaveBackup() {
    if (!wallet || !walletId) return;
    setBusy(true);
    setError(null);
    try {
      const backup = await exportWalletBackup(walletId);
      const path = await saveTextFileWithDialog(
        `${sanitizedFilename(backup.name)}-minisatoshi-wallet-v1.json`,
        `${backup.json}\n`,
      );
      if (path) setMessage(`Backup saved to ${path}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onSaveBsms() {
    if (!walletId) return;
    setBusy(true);
    setError(null);
    try {
      const bsms = await exportBsms(walletId);
      const path = await saveTextFileWithDialog(
        `${sanitizedFilename(wallet?.name ?? "wallet")}.bsms`,
        bsms.text,
      );
      if (path) {
        setMessage(
          `BSMS saved to ${path} — first address ${bsms.firstAddress || "(see file)"}`,
        );
      }
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  if (!wallet && !error) return <p className="muted">Loading…</p>;
  if (!wallet) return <pre className="error">{error}</pre>;

  const hwRemembered = hasRememberedSigningDevice(wallet, getHwFingerprint());
  const backTo = isHot
    ? `/hot-wallets/${id}/transactions`
    : `/wallets/${walletId}/transactions`;

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Share · {wallet.name}</h2>
          <p>
            Watch-only sharing — xpubs/descriptor only, never seed or xprv.
          </p>
        </div>
        <Link className="button-link" to={backTo}>
          Back
        </Link>
      </header>

      {error ? <pre className="error">{error}</pre> : null}
      {message ? <p className="status">{message}</p> : null}

      <div className="row-actions" style={{ marginBottom: "1rem" }}>
        <span className="badge watch-only">Watch-only</span>
        {hwRemembered ? (
          <span className="badge">HW fingerprint remembered</span>
        ) : (
          <span className="badge muted-badge">No signing device attached</span>
        )}
      </div>

      <div className="grid-2">
        <div className="panel center">
          <h3>Descriptor QR</h3>
          {qr ? <img src={qr} alt={`Descriptor QR chunk ${chunkIndex + 1}`} /> : <p>…</p>}
          {chunks.length > 1 ? (
            <>
              <p className="muted">
                Chunk {chunkIndex + 1} / {chunks.length} — scan all parts on the
                other device (MSDESC1 framing).
              </p>
              <div className="row-actions">
                <button
                  type="button"
                  className="secondary"
                  disabled={chunkIndex <= 0}
                  onClick={() => setChunkIndex((i) => Math.max(0, i - 1))}
                >
                  Previous
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={chunkIndex >= chunks.length - 1}
                  onClick={() =>
                    setChunkIndex((i) => Math.min(chunks.length - 1, i + 1))
                  }
                >
                  Next
                </button>
              </div>
            </>
          ) : (
            <p className="muted">Single QR — paste or scan into Import wallet.</p>
          )}
        </div>

        <div className="panel">
          <h3>Watch-only instructions</h3>
          <ol className="list compact">
            <li>Share the descriptor file, QR, or BSMS — not your seed.</li>
            <li>
              Recipient: Wallets → Import wallet → paste / load file (or
              reassemble multi-QR paste).
            </li>
            <li>
              They can sync balances and receive; signing still needs hardware,
              software keys, Core, Liana, or Nunchuk.
            </li>
            <li>
              Sparrow: fund a receive address only — not import/sign Miniscript
              wallets (docs/interop.md).
            </li>
          </ol>
          <p className="mono wrap">{wallet.descriptor}</p>
          <div className="row-actions">
            <button type="button" onClick={() => void onCopyDescriptor()}>
              Copy descriptor
            </button>
            <button
              type="button"
              className="secondary"
              onClick={() => void onSaveDescriptor()}
            >
              Save descriptor file
            </button>
            <button
              type="button"
              className="secondary"
              disabled={busy}
              onClick={() => void onSaveBackup()}
            >
              Save wallet backup
            </button>
            <button
              type="button"
              className="secondary"
              disabled={busy}
              onClick={() => void onSaveBsms()}
            >
              Save BSMS
            </button>
          </div>
        </div>
      </div>
    </section>
  );
}

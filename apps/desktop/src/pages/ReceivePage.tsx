import QRCode from "qrcode";
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  exportSparrowWallet,
  formatError,
  getVault,
  newReceiveAddress,
} from "../lib/api";
import { copyText } from "../lib/settings";
import { saveTextFileWithDialog, sanitizedFilename } from "../lib/download";
import type { AddressDto, SparrowExportDto, VaultDto } from "../lib/types";
import { useVault, useVaultIdFromRouteOrContext } from "../vault/VaultContext";
import { useT } from "../i18n/LocaleContext";

export function ReceivePage() {
  const t = useT();
  const id = useVaultIdFromRouteOrContext();
  const { setError, setMessage } = useVault();
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [address, setAddress] = useState<AddressDto | null>(null);
  const [qr, setQr] = useState<string | null>(null);
  const [sparrow, setSparrow] = useState<SparrowExportDto | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void getVault(id)
      .then(setVault)
      .catch((err) => setError(formatError(err)));
  }, [id]);

  async function deriveAddress() {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      const next = await newReceiveAddress(id);
      setAddress(next);
      setQr(await QRCode.toDataURL(next.address, { margin: 1, width: 220 }));
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void deriveAddress();
  }, [id]);

  async function onCopy(value: string, label: string) {
    await copyText(value);
    setMessage(`Copied ${label}`);
  }

  async function onSparrowExport() {
    setError(null);
    try {
      setSparrow(await exportSparrowWallet(id));
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onSaveDescriptorFile() {
    if (!vault) return;
    setError(null);
    try {
      const filename = `${sanitizedFilename(vault.name)}-descriptor.txt`;
      const path = await saveTextFileWithDialog(
        filename,
        `${vault.descriptor}\n`,
      );
      if (path) {
        setMessage(
          `Saved to ${path} — for Core / Nunchuk / Minisatoshi import (Sparrow: fund address only)`,
        );
      }
    } catch (err) {
      setError(formatError(err));
    }
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>{t("receive.title")}</h2>
          <p>{t("receive.subtitle")}</p>
          <p>{vault?.name ?? "Vault"} · Taproot address</p>
        </div>
        <Link className="button-link" to="../transactions" relative="path">
          {t("send.transactionsLink")}
        </Link>
      </header>

      <div className="grid-2">
        <div className="panel center">
          {qr ? <img src={qr} alt="Receive address QR" /> : <p>…</p>}
          {address ? (
            <>
              <p className="mono wrap">{address.address}</p>
              <p className="muted">Index {address.index}</p>
              <div className="row-actions">
                <button
                  type="button"
                  onClick={() => void onCopy(address.address, "address")}
                >
                  Copy address
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={busy}
                  onClick={() => void deriveAddress()}
                >
                  Next address
                </button>
              </div>
            </>
          ) : null}
        </div>

        <div className="panel">
          <h3>Descriptor backup</h3>
          <p className="muted">
            Fund by sending to the address on the left. For watch-only share use
            Vault → Share. Sign in Minisatoshi (HW/software), Bitcoin Core, or
            Nunchuk — Sparrow funds addresses only (see docs/interop.md).
          </p>
          {vault ? (
            <>
              <p className="mono wrap">{vault.descriptor}</p>
              <div className="row-actions">
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void onCopy(vault.descriptor, "descriptor")}
                >
                  Copy descriptor
                </button>
                <button
                  type="button"
                  onClick={() => void onSaveDescriptorFile()}
                >
                  Save descriptor file
                </button>
                <button type="button" onClick={() => void onSparrowExport()}>
                  Fund / interop notes
                </button>
              </div>
            </>
          ) : null}
          {sparrow ? (
            <div className="sparrow-box">
              <p>{sparrow.importInstructions}</p>
              <button
                type="button"
                className="secondary"
                onClick={() => void onCopy(sparrow.descriptor, "descriptor")}
              >
                Copy descriptor
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </section>
  );
}

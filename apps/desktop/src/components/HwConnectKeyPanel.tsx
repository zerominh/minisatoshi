import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import {
  formatError,
  getHwiStatus,
  hwGetXpub,
  listHwDevices,
} from "../lib/api";
import {
  defaultBip86AccountPath,
  originPathFromDerivation,
} from "../lib/hwPaths";
import { getHwiPath, setHwFingerprint, setHwiPath } from "../lib/settings";
import type { HwDeviceDto, NetworkName } from "../lib/types";

export type HwKeyFields = {
  xpub: string;
  fingerprint: string;
  origin_path: string;
};

type Props = {
  network: NetworkName;
  /** Fills XPUB / fingerprint / origin on the active key slot. */
  onApplied: (fields: HwKeyFields) => void;
  onError: (message: string | null) => void;
  onHint?: (message: string | null) => void;
};

/**
 * Enumerate HWI devices and pull account xpub into a Create-wallet key row.
 */
export function HwConnectKeyPanel({
  network,
  onApplied,
  onError,
  onHint,
}: Props) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [devices, setDevices] = useState<HwDeviceDto[]>([]);
  const [derivationPath, setDerivationPath] = useState(() =>
    defaultBip86AccountPath(network),
  );
  const [statusLine, setStatusLine] = useState<string | null>(null);

  useEffect(() => {
    setDerivationPath(defaultBip86AccountPath(network));
  }, [network]);

  async function refreshDevices() {
    setBusy(true);
    onError(null);
    onHint?.(null);
    try {
      const hwiPath = getHwiPath() || null;
      const status = await getHwiStatus(hwiPath);
      if (status.path && !getHwiPath()) {
        setHwiPath(status.path);
      }
      if (!status.available) {
        setDevices([]);
        setStatusLine(
          status.message ??
            "HWI not found — install under Settings → Signing devices.",
        );
        return;
      }
      const list = await listHwDevices(getHwiPath() || null);
      setDevices(list);
      setStatusLine(
        list.length === 0
          ? "No devices found — unlock the hardware wallet and try again."
          : `Found ${list.length} device(s). Approve xpub export on the device.`,
      );
    } catch (err) {
      setDevices([]);
      onError(formatError(err));
      setStatusLine(null);
    } finally {
      setBusy(false);
    }
  }

  async function openAndScan() {
    setOpen(true);
    await refreshDevices();
  }

  async function useDevice(fingerprint: string) {
    const fp = fingerprint.trim();
    if (!fp) return;
    setBusy(true);
    onError(null);
    try {
      const result = await hwGetXpub({
        fingerprint: fp,
        derivationPath: derivationPath.trim() || defaultBip86AccountPath(network),
        hwiPath: getHwiPath() || null,
      });
      const origin = originPathFromDerivation(result.derivationPath);
      setHwFingerprint(fp);
      onApplied({
        xpub: result.xpub.trim(),
        fingerprint: fp,
        origin_path: origin,
      });
      onHint?.(
        `Key filled from ${fp} @ ${result.derivationPath}`,
      );
      setStatusLine(`Applied ${fp}`);
    } catch (err) {
      onError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="hw-connect-key">
      <div className="row-actions">
        <button
          type="button"
          className="secondary"
          disabled={busy}
          onClick={() => void openAndScan()}
        >
          {busy && open ? "Scanning…" : "Connect hardware…"}
        </button>
        {open ? (
          <button
            type="button"
            className="secondary"
            disabled={busy}
            onClick={() => void refreshDevices()}
          >
            Refresh devices
          </button>
        ) : null}
        <Link className="button-link secondary" to="/settings">
          HWI settings
        </Link>
      </div>
      {open ? (
        <div className="form-grid hw-connect-key-body">
          <label>
            Derivation path (account xpub)
            <input
              className="mono"
              value={derivationPath}
              onChange={(e) => setDerivationPath(e.target.value)}
              placeholder={defaultBip86AccountPath(network)}
            />
          </label>
          {statusLine ? <p className="muted">{statusLine}</p> : null}
          {devices.length > 0 ? (
            <ul className="list compact">
              {devices.map((device) => (
                <li key={device.id} className="list-item">
                  <div>
                    <strong>
                      {device.model || device.deviceType} ·{" "}
                      <span className="mono">
                        {device.fingerprint || "—"}
                      </span>
                    </strong>
                    <div className="muted">
                      {device.deviceType}
                      {device.needsPin ? " · needs PIN" : ""}
                      {device.needsPassphrase ? " · needs passphrase" : ""}
                    </div>
                    {device.error ? (
                      <div className="error">{device.error}</div>
                    ) : null}
                  </div>
                  {device.fingerprint ? (
                    <button
                      type="button"
                      disabled={busy}
                      onClick={() => void useDevice(device.fingerprint)}
                    >
                      {busy ? "Reading…" : "Use for this key"}
                    </button>
                  ) : null}
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

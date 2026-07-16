import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useT } from "../i18n/LocaleContext";
import { formatError, getHwiStatus, listHwDevices } from "../lib/api";
import {
  formatNetwork,
  getHwiPath,
  setHwiPath,
  setHwFingerprint,
} from "../lib/settings";
import type { HwDeviceDto, NetworkName } from "../lib/types";

type Props = {
  network: NetworkName;
  fingerprint: string;
  onFingerprintChange: (fp: string) => void;
  /** Wallet key fingerprints — auto-select a matching connected device. */
  walletFingerprints?: string[];
  onError?: (message: string | null) => void;
  disabled?: boolean;
};

function normalizeFp(fp: string): string {
  return fp.trim().toLowerCase();
}

function pickAutoDevice(
  list: HwDeviceDto[],
  currentFp: string,
  walletFingerprints: string[],
): string | null {
  const withFp = list.filter((d) => d.fingerprint?.trim());
  const saved = normalizeFp(currentFp);
  if (saved && withFp.some((d) => normalizeFp(d.fingerprint) === saved)) {
    return null;
  }
  const walletSet = new Set(
    walletFingerprints.map(normalizeFp).filter(Boolean),
  );
  const policyMatch = withFp.find((d) =>
    walletSet.has(normalizeFp(d.fingerprint)),
  );
  if (policyMatch) return policyMatch.fingerprint;
  if (withFp.length === 1) return withFp[0].fingerprint;
  return null;
}

export function HwDevicePicker({
  network,
  fingerprint,
  onFingerprintChange,
  walletFingerprints = [],
  onError,
  disabled = false,
}: Props) {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [devices, setDevices] = useState<HwDeviceDto[]>([]);
  const [statusLine, setStatusLine] = useState<string | null>(null);
  const [scanned, setScanned] = useState(false);
  const fingerprintRef = useRef(fingerprint);
  fingerprintRef.current = fingerprint;
  const walletFpsRef = useRef(walletFingerprints);
  walletFpsRef.current = walletFingerprints;
  const autoPickedRef = useRef(false);

  const pickFingerprint = useCallback(
    (fp: string) => {
      const trimmed = fp.trim();
      onFingerprintChange(trimmed);
      setHwFingerprint(trimmed);
    },
    [onFingerprintChange],
  );

  const refreshDevices = useCallback(
    async (allowAutoPick: boolean) => {
      setBusy(true);
      onError?.(null);
      try {
        const hwiPath = getHwiPath() || null;
        const status = await getHwiStatus(hwiPath);
        if (status.path && !getHwiPath()) {
          setHwiPath(status.path);
        }
        if (!status.available) {
          setDevices([]);
          setStatusLine(status.message ?? t("settings.hwiNotFound"));
          return;
        }
        const list = await listHwDevices(getHwiPath() || null, network);
        setDevices(list);
        setScanned(true);
        setStatusLine(
          list.length === 0
            ? t("settings.noDevicesFound")
            : t("settings.foundDevices", { n: list.length }),
        );
        if (allowAutoPick && !autoPickedRef.current) {
          const auto = pickAutoDevice(
            list,
            fingerprintRef.current,
            walletFpsRef.current,
          );
          if (auto) {
            autoPickedRef.current = true;
            pickFingerprint(auto);
          }
        }
      } catch (err) {
        setDevices([]);
        onError?.(formatError(err));
        setStatusLine(null);
      } finally {
        setBusy(false);
      }
    },
    [network, onError, pickFingerprint, t],
  );

  useEffect(() => {
    autoPickedRef.current = false;
    void refreshDevices(true);
  }, [network, refreshDevices]);

  const selectable = devices.filter((d) => d.fingerprint?.trim());

  return (
    <div className="hw-connect-key">
      <div className="row-actions">
        <button
          type="button"
          className="secondary"
          disabled={disabled || busy}
          onClick={() => void refreshDevices(false)}
        >
          {busy ? t("send.hwScanning") : t("send.hwScanDevices")}
        </button>
        <Link className="button-link secondary" to="/settings">
          {t("send.hwSettingsLink")}
        </Link>
      </div>
      <p className="muted">
        {t("send.hwNetworkHint", { network: formatNetwork(network) })}
      </p>
      {statusLine ? <p className="muted">{statusLine}</p> : null}
      {scanned && selectable.length > 0 ? (
        <label>
          {t("send.hwSelectDevice")}
          <select
            value={fingerprint}
            disabled={disabled || busy}
            onChange={(e) => pickFingerprint(e.target.value)}
          >
            <option value="">{t("send.hwSelectPlaceholder")}</option>
            {selectable.map((device) => (
              <option key={device.id} value={device.fingerprint}>
                {device.model || device.deviceType} · {device.fingerprint}
                {device.needsPin ? ` · ${t("settings.needsPin")}` : ""}
                {device.needsPassphrase
                  ? ` · ${t("settings.needsPassphrase")}`
                  : ""}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      {selectable.length > 0 ? (
        <ul className="list compact">
          {selectable.map((device) => (
            <li key={device.id} className="list-item">
              <div>
                <strong>
                  {device.model || device.deviceType} ·{" "}
                  <span className="mono">{device.fingerprint}</span>
                </strong>
                <div className="muted">
                  {device.deviceType}
                  {device.needsPin ? ` · ${t("settings.needsPin")}` : ""}
                  {device.needsPassphrase
                    ? ` · ${t("settings.needsPassphrase")}`
                    : ""}
                </div>
                {device.error ? (
                  <div className="error">{device.error}</div>
                ) : null}
              </div>
              <button
                type="button"
                className={
                  normalizeFp(device.fingerprint) === normalizeFp(fingerprint)
                    ? "btn-ok"
                    : "secondary"
                }
                disabled={disabled || busy}
                onClick={() => pickFingerprint(device.fingerprint)}
              >
                {normalizeFp(device.fingerprint) === normalizeFp(fingerprint)
                  ? t("send.hwSelected")
                  : t("common.use")}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
      <label>
        {t("settings.preferredFingerprint")}
        <input
          className="mono"
          value={fingerprint}
          disabled={disabled || busy}
          onChange={(e) => pickFingerprint(e.target.value)}
          placeholder="a1b2c3d4"
        />
      </label>
    </div>
  );
}

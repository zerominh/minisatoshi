import { FormEvent, useEffect, useState } from "react";
import {
  appVersion,
  formatError,
  hwGetXpub,
  listHwDevices,
  listServerPresets,
} from "../lib/api";
import {
  formatNetwork,
  getEsploraUrl,
  getHwiPath,
  getHwFingerprint,
  getPreferredNetwork,
  setEsploraUrl,
  setHwiPath,
  setHwFingerprint,
  setPreferredNetwork,
} from "../lib/settings";
import type { HwDeviceDto, NetworkName, ServerPresetDto } from "../lib/types";

export function SettingsPage() {
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [esploraUrl, setUrl] = useState(getEsploraUrl());
  const [hwiPath, setHwiPathState] = useState(getHwiPath());
  const [hwFingerprint, setHwFingerprintState] = useState(getHwFingerprint());
  const [devices, setDevices] = useState<HwDeviceDto[]>([]);
  const [xpubPath, setXpubPath] = useState("m/86'/1'/0'");
  const [xpubResult, setXpubResult] = useState<string | null>(null);
  const [presets, setPresets] = useState<ServerPresetDto[]>([]);
  const [version, setVersion] = useState<string>("…");
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void appVersion().then(setVersion).catch(() => setVersion("unknown"));
  }, []);

  useEffect(() => {
    void listServerPresets(network)
      .then(setPresets)
      .catch((err) => setError(formatError(err)));
  }, [network]);

  function onSave(event: FormEvent) {
    event.preventDefault();
    setPreferredNetwork(network);
    setEsploraUrl(esploraUrl);
    setHwiPath(hwiPath);
    setHwFingerprint(hwFingerprint);
    setMessage("Settings saved locally.");
  }

  async function onRefreshDevices() {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      setHwiPath(hwiPath);
      const list = await listHwDevices(hwiPath || null);
      setDevices(list);
      setMessage(
        list.length === 0
          ? "No devices found — install HWI and connect a wallet."
          : `Found ${list.length} device(s).`,
      );
    } catch (err) {
      setError(formatError(err));
      setDevices([]);
    } finally {
      setBusy(false);
    }
  }

  async function onGetXpub(fingerprint: string) {
    setBusy(true);
    setError(null);
    try {
      const result = await hwGetXpub({
        fingerprint,
        derivationPath: xpubPath,
        hwiPath: hwiPath || null,
      });
      setXpubResult(result.xpub);
      setHwFingerprintState(fingerprint);
      setHwFingerprint(fingerprint);
      setMessage(`xpub for ${fingerprint} @ ${result.derivationPath}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Settings</h2>
          <p>
            Network defaults, Esplora backends, and hardware signing (HWI).
          </p>
        </div>
        <p className="muted">Minisatoshi v{version}</p>
      </header>

      <form className="panel form-grid" onSubmit={onSave}>
        <label>
          Preferred network (new wallets)
          <select
            value={network}
            onChange={(e) => setNetwork(e.target.value as NetworkName)}
          >
            <option value="testnet">Testnet3</option>
            <option value="testnet4">Testnet4</option>
            <option value="signet">Signet</option>
            <option value="regtest">Regtest</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </label>
        <label>
          Esplora URL override (optional)
          <input
            value={esploraUrl}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://blockstream.info/testnet/api"
          />
        </label>
        <button type="submit">Save</button>
      </form>

      <div className="panel form-grid">
        <h3>Signing devices (HWI)</h3>
        <p className="muted">
          Requires the{" "}
          <a
            href="https://github.com/bitcoin-core/HWI"
            target="_blank"
            rel="noreferrer"
          >
            HWI
          </a>{" "}
          binary on PATH, or set an absolute path below. Secrets never leave the
          device.
        </p>
        <label>
          HWI binary path (optional)
          <input
            className="mono"
            value={hwiPath}
            onChange={(e) => setHwiPathState(e.target.value)}
            placeholder="hwi or C:\path\to\hwi.exe"
          />
        </label>
        <label>
          Preferred device fingerprint (for Send)
          <input
            className="mono"
            value={hwFingerprint}
            onChange={(e) => setHwFingerprintState(e.target.value)}
            placeholder="a1b2c3d4"
          />
        </label>
        <label>
          Derivation path for xpub
          <input
            className="mono"
            value={xpubPath}
            onChange={(e) => setXpubPath(e.target.value)}
            placeholder="m/86'/1'/0'"
          />
        </label>
        <div className="row-actions">
          <button
            type="button"
            disabled={busy}
            onClick={() => void onRefreshDevices()}
          >
            Refresh devices
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => {
              setHwiPath(hwiPath);
              setHwFingerprint(hwFingerprint);
              setMessage("HWI path and fingerprint saved.");
            }}
          >
            Save device settings
          </button>
        </div>
        {devices.length > 0 ? (
          <ul className="list">
            {devices.map((device) => (
              <li key={device.id} className="list-item">
                <div>
                  <strong>
                    {device.model || device.deviceType} ·{" "}
                    <span className="mono">{device.fingerprint || "—"}</span>
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
                <div className="row-actions">
                  {device.fingerprint ? (
                    <>
                      <button
                        type="button"
                        className="secondary"
                        disabled={busy}
                        onClick={() => {
                          setHwFingerprintState(device.fingerprint);
                          setHwFingerprint(device.fingerprint);
                          setMessage(`Using fingerprint ${device.fingerprint}`);
                        }}
                      >
                        Use
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void onGetXpub(device.fingerprint)}
                      >
                        Get xpub
                      </button>
                    </>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">No devices enumerated yet.</p>
        )}
        {xpubResult ? (
          <label>
            Last xpub
            <textarea className="mono" rows={2} readOnly value={xpubResult} />
          </label>
        ) : null}
      </div>

      {message ? <p className="status">{message}</p> : null}
      {error ? <pre className="error">{error}</pre> : null}

      <div className="panel">
        <h3>Sparrow-compatible server presets</h3>
        <ul className="list">
          {presets.map((preset) => (
            <li key={`${preset.backend}-${preset.url}`} className="list-item">
              <div>
                <strong>{preset.label}</strong>
                <div className="muted">
                  {preset.backend} · {formatNetwork(preset.network)}
                </div>
                <div className="mono wrap">{preset.url}</div>
              </div>
              {preset.backend === "esplora" ? (
                <button
                  type="button"
                  className="secondary"
                  onClick={() => {
                    setUrl(preset.url);
                    setEsploraUrl(preset.url);
                    setMessage(`Using ${preset.label}`);
                  }}
                >
                  Use
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}

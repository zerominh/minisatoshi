import { FormEvent, useEffect, useState } from "react";
import {
  appVersion,
  ensureHwiInstalled,
  formatError,
  getHwiStatus,
  hwGetXpub,
  listHwDevices,
  listServerPresets,
} from "../lib/api";
import { useFlash } from "../flash/FlashContext";
import { LOCALES, type Locale } from "../i18n/en";
import { useLocale, useT } from "../i18n/LocaleContext";
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
import type {
  HwDeviceDto,
  HwStatusDto,
  NetworkName,
  ServerPresetDto,
} from "../lib/types";

export function SettingsPage() {
  const t = useT();
  const { locale, setLocale } = useLocale();
  const { setError, setMessage } = useFlash();
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [esploraUrl, setUrl] = useState(getEsploraUrl());
  const [hwiPath, setHwiPathState] = useState(getHwiPath());
  const [hwFingerprint, setHwFingerprintState] = useState(getHwFingerprint());
  const [hwiStatus, setHwiStatus] = useState<HwStatusDto | null>(null);
  const [devices, setDevices] = useState<HwDeviceDto[]>([]);
  const [xpubPath, setXpubPath] = useState("m/86'/1'/0'");
  const [xpubResult, setXpubResult] = useState<string | null>(null);
  const [presets, setPresets] = useState<ServerPresetDto[]>([]);
  const [version, setVersion] = useState<string>("…");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void appVersion().then(setVersion).catch(() => setVersion("unknown"));
  }, []);

  useEffect(() => {
    void listServerPresets(network)
      .then(setPresets)
      .catch((err) => setError(formatError(err)));
  }, [network, setError]);

  useEffect(() => {
    void getHwiStatus(hwiPath || null)
      .then(setHwiStatus)
      .catch(() => setHwiStatus(null));
  }, []);

  function onSave(event: FormEvent) {
    event.preventDefault();
    setPreferredNetwork(network);
    setEsploraUrl(esploraUrl);
    setHwiPath(hwiPath);
    setHwFingerprint(hwFingerprint);
    setMessage(t("settings.saved"));
  }

  async function refreshHwiStatus() {
    const status = await getHwiStatus(hwiPath || null);
    setHwiStatus(status);
    if (status.path) {
      setHwiPathState(status.path);
      setHwiPath(status.path);
    }
    return status;
  }

  async function onInstallHwi() {
    setBusy(true);
    setError(null);
    setMessage(
      t("settings.downloadingHwi", {
        version: hwiStatus?.pinnedVersion ?? "",
      }),
    );
    try {
      const status = await ensureHwiInstalled(hwiPath || null);
      setHwiStatus(status);
      if (status.path) {
        setHwiPathState(status.path);
        setHwiPath(status.path);
      }
      setMessage(
        status.message ??
          `${t("settings.hwiReady")}${status.version ? ` · ${status.version}` : ""}`,
      );
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  async function onRefreshDevices() {
    setBusy(true);
    setError(null);
    setMessage(null);
    try {
      setHwiPath(hwiPath);
      await refreshHwiStatus().catch(() => undefined);
      const list = await listHwDevices(hwiPath || null, network);
      const status = await getHwiStatus(hwiPath || null);
      setHwiStatus(status);
      if (status.path && !getHwiPath()) {
        setHwiPathState(status.path);
        setHwiPath(status.path);
      }
      setDevices(list);
      setMessage(
        list.length === 0
          ? t("settings.noDevicesFound")
          : t("settings.foundDevices", { n: list.length }),
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
        network,
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
          <h2>{t("settings.title")}</h2>
          <p>{t("settings.subtitle")}</p>
        </div>
        <p className="muted">Minisatoshi v{version}</p>
      </header>

      <div className="panel form-grid">
        <h3>{t("settings.language")}</h3>
        <p className="muted">{t("settings.languageHint")}</p>
        <label>
          {t("settings.language")}
          <select
            value={locale}
            onChange={(e) => setLocale(e.target.value as Locale)}
          >
            {LOCALES.map((item) => (
              <option key={item.id} value={item.id}>
                {t(item.labelKey)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <form className="panel form-grid" onSubmit={onSave}>
        <label>
          {t("settings.preferredNetwork")}
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
          {t("settings.esploraOverride")}
          <input
            value={esploraUrl}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://blockstream.info/testnet/api"
          />
        </label>
        <button type="submit">{t("common.save")}</button>
      </form>

      <div className="panel form-grid">
        <h3>{t("settings.signingDevices")}</h3>
        <p className="muted">{t("settings.hwiHint")}</p>
        {hwiStatus ? (
          <p className={hwiStatus.available ? "status" : "muted"}>
            {hwiStatus.available
              ? `${t("settings.hwiReady")} · ${hwiStatus.version ?? "unknown"} · ${hwiStatus.source ?? ""} · ${hwiStatus.path ?? ""}`
              : (hwiStatus.message ?? t("settings.hwiNotFound"))}
          </p>
        ) : null}
        <label>
          {t("settings.hwiPath")}
          <input
            className="mono"
            value={hwiPath}
            onChange={(e) => setHwiPathState(e.target.value)}
            placeholder="auto / PATH / app-managed"
          />
        </label>
        <label>
          {t("settings.preferredFingerprint")}
          <input
            className="mono"
            value={hwFingerprint}
            onChange={(e) => setHwFingerprintState(e.target.value)}
            placeholder="a1b2c3d4"
          />
        </label>
        <label>
          {t("settings.derivationPath")}
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
            onClick={() => void onInstallHwi()}
          >
            {hwiStatus?.available
              ? t("settings.verifyHwi")
              : t("settings.installHwi")}
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => void onRefreshDevices()}
          >
            {t("settings.refreshDevices")}
          </button>
          <button
            type="button"
            className="secondary"
            onClick={() => {
              setHwiPath(hwiPath);
              setHwFingerprint(hwFingerprint);
              setMessage(t("settings.deviceSettingsSaved"));
            }}
          >
            {t("settings.saveDevice")}
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
                    {device.needsPin ? ` · ${t("settings.needsPin")}` : ""}
                    {device.needsPassphrase
                      ? ` · ${t("settings.needsPassphrase")}`
                      : ""}
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
                          setMessage(
                            t("settings.usingFingerprint", {
                              fp: device.fingerprint,
                            }),
                          );
                        }}
                      >
                        {t("common.use")}
                      </button>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void onGetXpub(device.fingerprint)}
                      >
                        {t("settings.getXpub")}
                      </button>
                    </>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p className="muted">{t("settings.noDevicesYet")}</p>
        )}
        {xpubResult ? (
          <label>
            {t("settings.lastXpub")}
            <textarea className="mono" rows={2} readOnly value={xpubResult} />
          </label>
        ) : null}
      </div>

      <div className="panel">
        <h3>{t("settings.serverPresets")}</h3>
        <p className="muted">{t("settings.serverPresetsHint")}</p>
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
                    setMessage(
                      t("settings.usingPreset", { label: preset.label }),
                    );
                  }}
                >
                  {t("common.use")}
                </button>
              ) : null}
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}

import { FormEvent, useEffect, useState } from "react";
import { formatError, listServerPresets } from "../lib/api";
import {
  getEsploraUrl,
  getPreferredNetwork,
  setEsploraUrl,
  setPreferredNetwork,
} from "../lib/settings";
import type { NetworkName, ServerPresetDto } from "../lib/types";

export function SettingsPage() {
  const [network, setNetwork] = useState<NetworkName>(getPreferredNetwork());
  const [esploraUrl, setUrl] = useState(getEsploraUrl());
  const [presets, setPresets] = useState<ServerPresetDto[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void listServerPresets(network)
      .then(setPresets)
      .catch((err) => setError(formatError(err)));
  }, [network]);

  function onSave(event: FormEvent) {
    event.preventDefault();
    setPreferredNetwork(network);
    setEsploraUrl(esploraUrl);
    setMessage("Settings saved locally.");
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Settings</h2>
          <p>Network defaults and blockchain backends (Esplora / Sparrow presets).</p>
        </div>
      </header>

      <form className="panel form-grid" onSubmit={onSave}>
        <label>
          Preferred network (new wallets)
          <select
            value={network}
            onChange={(e) => setNetwork(e.target.value as NetworkName)}
          >
            <option value="testnet">Testnet</option>
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
                  {preset.backend} · {preset.network}
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

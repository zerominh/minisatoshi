import { useEffect, useState } from "react";
import { formatError, listAddresses } from "../lib/api";
import { copyText } from "../lib/settings";
import type { AddressDto } from "../lib/types";
import { useVault } from "../vault/VaultContext";

export function VaultAddressesPage() {
  const { vaultId, busy: vaultBusy, setError, setMessage } = useVault();
  const [addresses, setAddresses] = useState<AddressDto[]>([]);
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setBusy(true);
    try {
      setAddresses(await listAddresses(vaultId));
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, [vaultId]);

  async function onCopy(address: string) {
    await copyText(address);
    setMessage("Address copied");
  }

  const receive = addresses.filter((a) => !a.isChange);
  const change = addresses.filter((a) => a.isChange);

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Addresses</h2>
          <p>Derived receive &amp; change addresses stored for this vault.</p>
        </div>
        <button
          type="button"
          className="secondary"
          disabled={busy || vaultBusy}
          onClick={() => void refresh()}
        >
          Refresh
        </button>
      </header>

      <div className="grid-2">
        <div className="panel">
          <h3>Receive ({receive.length})</h3>
          {receive.length === 0 ? (
            <p className="muted">
              None yet — open Receive to derive the next address.
            </p>
          ) : (
            <ul className="list compact">
              {receive.map((addr) => (
                <li key={`r-${addr.index}-${addr.address}`}>
                  <button
                    type="button"
                    className="linkish mono wrap"
                    onClick={() => void onCopy(addr.address)}
                    title="Copy"
                  >
                    [{addr.index}] {addr.address}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="panel">
          <h3>Change ({change.length})</h3>
          {change.length === 0 ? (
            <p className="muted">No change addresses yet.</p>
          ) : (
            <ul className="list compact">
              {change.map((addr) => (
                <li key={`c-${addr.index}-${addr.address}`}>
                  <button
                    type="button"
                    className="linkish mono wrap"
                    onClick={() => void onCopy(addr.address)}
                    title="Copy"
                  >
                    [{addr.index}] {addr.address}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
}

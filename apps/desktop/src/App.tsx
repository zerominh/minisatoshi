import { useState } from "react";
import {
  createVault,
  createWallet,
  listWallets,
  newReceiveAddress,
} from "./lib/api";
import type { AddressDto, PolicyConfig, VaultDto, WalletDto } from "./lib/types";

/** rust-miniscript test-vector xpubs (same as backend unit tests). */
const DEMO_POLICY: PolicyConfig = {
  version: 1,
  network: "testnet",
  script_type: "taproot",
  keys: [
    {
      id: "A",
      role: "investor",
      xpub: "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL",
      fingerprint: "78412e3a",
      origin_path: "44'/0'/0'",
    },
    {
      id: "B",
      role: "manager",
      xpub: "xpub6BgBgsespWvERF3LHQu6CnqdvfEvtMcQjYrcRzx53QJjSxarj2afYWcLteoGVky7D3UKDP9QyrLprQ3VCECoY49yfdDEHGCtMMj92pReUsQ",
      fingerprint: "73c5da0a",
      origin_path: "86'/0'/0'",
    },
    {
      id: "C",
      role: "recovery",
      xpub: "xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V",
      fingerprint: "73c5da0a",
      origin_path: "84'/0'/0'",
    },
  ],
  policy: {
    primary: "(A && B) || (A && C)",
    fallback: { after: "4y", allow: "A" },
  },
};

function App() {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [wallet, setWallet] = useState<WalletDto | null>(null);
  const [vault, setVault] = useState<VaultDto | null>(null);
  const [address, setAddress] = useState<AddressDto | null>(null);
  const [walletCount, setWalletCount] = useState<number | null>(null);

  async function onCreateDemoVault() {
    setBusy(true);
    setError(null);
    try {
      const created = await createWallet({
        name: `Demo ${new Date().toLocaleTimeString()}`,
        network: "testnet",
      });
      setWallet(created);

      const createdVault = await createVault({
        walletId: created.id,
        name: "ABC Vault",
        policy: DEMO_POLICY,
      });
      setVault(createdVault);

      const receive = await newReceiveAddress(createdVault.id);
      setAddress(receive);

      const wallets = await listWallets();
      setWalletCount(wallets.length);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app">
      <aside className="sidebar">
        <h1>Minisatoshi</h1>
        <nav>
          <a href="#">Wallets</a>
          <a href="#">Vaults</a>
          <a href="#">Transactions</a>
          <a href="#">Settings</a>
        </nav>
      </aside>
      <main className="content">
        <h2>Bitcoin Vault Engine</h2>
        <p>
          Offline desktop app for creating and managing Bitcoin vaults with
          Miniscript.
        </p>
        <p className="status">Sprint 6 — Tauri IPC bridge ready.</p>

        <section className="ipc-panel">
          <h3>IPC smoke test</h3>
          <p>
            Creates a testnet wallet + ABC vault and derives the first receive
            address via Rust crates.
          </p>
          <button
            type="button"
            disabled={busy}
            onClick={() => void onCreateDemoVault()}
          >
            {busy ? "Working…" : "Create demo vault"}
          </button>

          {error ? <pre className="error">{error}</pre> : null}

          {wallet ? (
            <dl className="result">
              <dt>Wallet</dt>
              <dd>
                {wallet.name} · {wallet.network} · {wallet.id.slice(0, 8)}…
              </dd>
              {vault ? (
                <>
                  <dt>Vault</dt>
                  <dd>
                    {vault.name} · {vault.scriptType}
                  </dd>
                  <dt>Descriptor</dt>
                  <dd className="mono">{vault.descriptor}</dd>
                </>
              ) : null}
              {address ? (
                <>
                  <dt>Receive address</dt>
                  <dd className="mono">{address.address}</dd>
                </>
              ) : null}
              {walletCount != null ? (
                <>
                  <dt>Wallets in store</dt>
                  <dd>{walletCount}</dd>
                </>
              ) : null}
            </dl>
          ) : null}
        </section>
      </main>
    </div>
  );
}

export default App;

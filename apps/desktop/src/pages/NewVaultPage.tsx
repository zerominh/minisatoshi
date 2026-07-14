import { FormEvent, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  compileVaultDescriptor,
  createVault,
  formatError,
  listWallets,
} from "../lib/api";
import {
  formatNetwork,
  getActiveWalletId,
  getPreferredNetwork,
  setActiveWalletId,
} from "../lib/settings";
import {
  durationToBlocks,
  formatDuration,
  type TimelockUnit,
} from "../lib/duration";
import type { KeyConfig, NetworkName, PolicyConfig } from "../lib/types";

type Step = 1 | 2 | 3 | 4 | 5;

const emptyKey = (id: string, role: KeyConfig["role"]): KeyConfig => ({
  id,
  role,
  xpub: "",
  fingerprint: "",
  origin_path: "",
});

export function NewVaultPage() {
  const navigate = useNavigate();
  const [step, setStep] = useState<Step>(1);
  const [walletId, setWalletId] = useState(getActiveWalletId() ?? "");
  const [wallets, setWallets] = useState<
    { id: string; name: string; network: NetworkName }[]
  >([]);
  const [name, setName] = useState("ABC Vault");
  const [investor, setInvestor] = useState(emptyKey("A", "investor"));
  const [manager, setManager] = useState(emptyKey("B", "manager"));
  const [recovery, setRecovery] = useState(emptyKey("C", "recovery"));
  const [timelockAmount, setTimelockAmount] = useState(4);
  const [timelockUnit, setTimelockUnit] = useState<TimelockUnit>("y");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    void listWallets()
      .then((items) => {
        setWallets(items);
        if (!walletId && items[0]) {
          setWalletId(items[0].id);
          setActiveWalletId(items[0].id);
        }
      })
      .catch((err) => setError(formatError(err)));
  }, []);

  const network =
    wallets.find((w) => w.id === walletId)?.network ?? getPreferredNetwork();

  const after = formatDuration(timelockAmount, timelockUnit);
  const timelockBlocks = durationToBlocks(timelockAmount, timelockUnit);

  const policy: PolicyConfig = useMemo(
    () => ({
      version: 1,
      network,
      script_type: "taproot",
      keys: [
        {
          ...investor,
          origin_path: investor.origin_path || undefined,
        },
        {
          ...manager,
          origin_path: manager.origin_path || undefined,
        },
        {
          ...recovery,
          origin_path: recovery.origin_path || undefined,
        },
      ],
      policy: {
        primary: "(A && B) || (A && C)",
        fallback: { after, allow: "A" },
      },
    }),
    [investor, manager, recovery, network, after],
  );

  function validateKey(key: KeyConfig, label: string): string | null {
    if (!key.xpub.trim()) return `${label}: xpub is required`;
    if (!/^[0-9a-fA-F]{8}$/.test(key.fingerprint.trim())) {
      return `${label}: fingerprint must be 8 hex chars`;
    }
    return null;
  }

  function canAdvance(): string | null {
    if (!walletId) return "Select a wallet";
    if (step === 1) return validateKey(investor, "Investor");
    if (step === 2) return validateKey(manager, "Manager");
    if (step === 3) return validateKey(recovery, "Recovery");
    if (step === 4) {
      if (!Number.isInteger(timelockAmount) || timelockAmount < 1) {
        return "Timelock amount must be at least 1";
      }
      if (timelockBlocks < 1) return "Timelock must be at least 1 block";
      if (timelockBlocks > 10 * 52_560) {
        return "Timelock is too large (max ~10 years)";
      }
    }
    if (step === 5 && !name.trim()) return "Vault name is required";
    return null;
  }

  async function onPreview() {
    setError(null);
    try {
      const result = await compileVaultDescriptor(policy);
      setPreview(result.descriptor);
    } catch (err) {
      setError(formatError(err));
    }
  }

  async function onSubmit(event: FormEvent) {
    event.preventDefault();
    const problem = canAdvance();
    if (problem) {
      setError(problem);
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const vault = await createVault({
        walletId,
        name,
        policy,
      });
      navigate(`/vaults/${vault.id}`);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setBusy(false);
    }
  }

  function renderKeyForm(
    title: string,
    key: KeyConfig,
    setKey: (value: KeyConfig) => void,
  ) {
    return (
      <div className="form-grid">
        <h3>{title}</h3>
        <label>
          XPUB
          <textarea
            rows={3}
            value={key.xpub}
            onChange={(e) => setKey({ ...key, xpub: e.target.value.trim() })}
            placeholder="xpub… or tpub…"
            required
          />
        </label>
        <label>
          Fingerprint (8 hex)
          <input
            value={key.fingerprint}
            onChange={(e) =>
              setKey({ ...key, fingerprint: e.target.value.trim() })
            }
            placeholder="78412e3a"
            required
          />
        </label>
        <label>
          Origin path (optional)
          <input
            value={key.origin_path ?? ""}
            onChange={(e) =>
              setKey({ ...key, origin_path: e.target.value.trim() })
            }
            placeholder="86'/0'/0'"
          />
        </label>
      </div>
    );
  }

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Create vault</h2>
          <p>ABC investor / manager / recovery · Taproot · optional inheritance.</p>
        </div>
      </header>

      <div className="steps">
        {[1, 2, 3, 4, 5].map((value) => (
          <span
            key={value}
            className={value === step ? "step active" : "step"}
          >
            {value}
          </span>
        ))}
      </div>

      <form className="panel" onSubmit={(e) => void onSubmit(e)}>
        {step < 5 ? (
          <>
            {step === 1 && (
              <>
                <label>
                  Wallet
                  <select
                    value={walletId}
                    onChange={(e) => {
                      setWalletId(e.target.value);
                      setActiveWalletId(e.target.value);
                    }}
                    required
                  >
                    <option value="" disabled>
                      Select wallet
                    </option>
                    {wallets.map((wallet) => (
                      <option key={wallet.id} value={wallet.id}>
                        {wallet.name} ({formatNetwork(wallet.network)})
                      </option>
                    ))}
                  </select>
                </label>
                {renderKeyForm("Step 1 · Investor key", investor, setInvestor)}
              </>
            )}
            {step === 2 &&
              renderKeyForm("Step 2 · Manager key", manager, setManager)}
            {step === 3 &&
              renderKeyForm("Step 3 · Recovery key", recovery, setRecovery)}
            {step === 4 && (
              <div className="form-grid">
                <h3>Step 4 · Inheritance timelock</h3>
                <p className="muted">
                  After this relative delay, investor (A) alone can spend via the
                  fallback path (`older(N)`).
                </p>
                <label>
                  Amount
                  <input
                    type="number"
                    min={1}
                    step={1}
                    value={timelockAmount}
                    onChange={(e) => setTimelockAmount(Number(e.target.value))}
                    required
                  />
                </label>
                <label>
                  Unit
                  <select
                    value={timelockUnit}
                    onChange={(e) =>
                      setTimelockUnit(e.target.value as TimelockUnit)
                    }
                  >
                    <option value="d">Days</option>
                    <option value="w">Weeks</option>
                    <option value="y">Years</option>
                    <option value="b">Blocks</option>
                  </select>
                </label>
                <p className="mono">
                  Policy: <strong>{after}</strong>
                  {" · "}
                  <strong>{timelockBlocks.toLocaleString()}</strong> blocks
                  {timelockUnit !== "b"
                    ? ` (≈ ${
                        timelockUnit === "d"
                          ? `${timelockAmount} × 144`
                          : timelockUnit === "w"
                            ? `${timelockAmount} × 1008`
                            : `${timelockAmount} × 52560`
                      })`
                    : null}
                </p>
              </div>
            )}

            <div className="row-actions">
              {step > 1 ? (
                <button
                  type="button"
                  className="secondary"
                  onClick={() => setStep((step - 1) as Step)}
                >
                  Back
                </button>
              ) : null}
              <button
                type="button"
                onClick={() => {
                  const problem = canAdvance();
                  if (problem) {
                    setError(problem);
                    return;
                  }
                  setError(null);
                  setStep((step + 1) as Step);
                }}
              >
                Next
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="form-grid">
              <h3>Step 5 · Review & generate</h3>
              <label>
                Vault name
                <input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  required
                />
              </label>
              <pre className="mono review">{JSON.stringify(policy, null, 2)}</pre>
              <div className="row-actions">
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void onPreview()}
                >
                  Preview descriptor
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => setStep(4)}
                >
                  Back
                </button>
                <button type="submit" disabled={busy}>
                  {busy ? "Creating…" : "Generate vault"}
                </button>
              </div>
              {preview ? (
                <p className="mono wrap">
                  <strong>Descriptor:</strong> {preview}
                </p>
              ) : null}
            </div>
          </>
        )}
      </form>

      {error ? <pre className="error">{error}</pre> : null}
    </section>
  );
}

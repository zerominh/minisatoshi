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
import { durationToBlocks, type TimelockUnit } from "../lib/duration";
import type { KeyConfig, KeyRole, NetworkName } from "../lib/types";
import {
  POLICY_TEMPLATES,
  buildPolicyConfig,
  emptyKey,
  keysFromTemplate,
  multiManagerPrimary,
  nextKeyId,
  type PolicyTemplate,
  type RecoveryPathDraft,
  type TemplateId,
} from "../lib/policyTemplates";

type Step = 1 | 2 | 3 | 4 | 5;

const ROLE_OPTIONS: KeyRole[] = [
  "investor",
  "manager",
  "recovery",
  "cosigner",
  "other",
];

export function NewVaultPage() {
  const navigate = useNavigate();
  const [step, setStep] = useState<Step>(1);
  const [walletId, setWalletId] = useState(getActiveWalletId() ?? "");
  const [wallets, setWallets] = useState<
    { id: string; name: string; network: NetworkName }[]
  >([]);
  const [templateId, setTemplateId] = useState<TemplateId>("abc");
  const [name, setName] = useState("ABC Vault");
  const [keys, setKeys] = useState<KeyConfig[]>(() =>
    keysFromTemplate(POLICY_TEMPLATES[0]),
  );
  const [primary, setPrimary] = useState(POLICY_TEMPLATES[0].defaultPrimary);
  const [recoveryPaths, setRecoveryPaths] = useState<RecoveryPathDraft[]>([
    { amount: 4, unit: "y", allow: "A" },
  ]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);

  const template =
    POLICY_TEMPLATES.find((t) => t.id === templateId) ?? POLICY_TEMPLATES[0];

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

  const policy = useMemo(
    () =>
      buildPolicyConfig({
        network,
        keys,
        primary,
        recoveryPaths,
      }),
    [network, keys, primary, recoveryPaths],
  );

  function applyTemplate(next: PolicyTemplate) {
    setTemplateId(next.id);
    setKeys(keysFromTemplate(next));
    setPrimary(next.defaultPrimary);
    setName(next.label);
    if (next.defaultFallback) {
      setRecoveryPaths([
        {
          amount: next.defaultFallback.amount,
          unit: next.defaultFallback.unit,
          allow: next.defaultFallback.allow,
        },
      ]);
    } else {
      setRecoveryPaths([]);
    }
    setPreview(null);
  }

  function syncMultiManagerPrimary(nextKeys: KeyConfig[]) {
    if (templateId === "multi_manager") {
      setPrimary(multiManagerPrimary(nextKeys.map((k) => k.id)));
    }
  }

  function updateKey(index: number, patch: Partial<KeyConfig>) {
    setKeys((prev) => {
      const next = prev.map((k, i) => (i === index ? { ...k, ...patch } : k));
      return next;
    });
  }

  function addKey(role: KeyRole = "manager") {
    setKeys((prev) => {
      const next = [...prev, emptyKey(nextKeyId(prev), role)];
      syncMultiManagerPrimary(next);
      return next;
    });
  }

  function removeKey(index: number) {
    setKeys((prev) => {
      if (prev.length <= 1) return prev;
      const next = prev.filter((_, i) => i !== index);
      syncMultiManagerPrimary(next);
      return next;
    });
  }

  function validateKey(key: KeyConfig, label: string): string | null {
    if (!key.id.trim()) return `${label}: id is required`;
    if (!key.xpub.trim()) return `${label}: xpub is required`;
    if (!/^[0-9a-fA-F]{8}$/.test(key.fingerprint.trim())) {
      return `${label}: fingerprint must be 8 hex chars`;
    }
    return null;
  }

  function canAdvance(): string | null {
    if (!walletId) return "Select a wallet";
    if (step === 2) {
      const ids = new Set<string>();
      for (const key of keys) {
        const label = `Key ${key.id || "?"}`;
        const err = validateKey(key, label);
        if (err) return err;
        if (ids.has(key.id)) return `Duplicate key id: ${key.id}`;
        ids.add(key.id);
      }
      if (keys.length < 1) return "Add at least one key";
    }
    if (step === 3) {
      if (!primary.trim()) return "Primary expression is required";
    }
    if (step === 4) {
      for (const path of recoveryPaths) {
        if (!path.allow.trim()) return "Each recovery path needs an allow expression";
        if (!Number.isInteger(path.amount) || path.amount < 1) {
          return "Recovery timelock amount must be at least 1";
        }
        const blocks = durationToBlocks(path.amount, path.unit);
        if (blocks < 1) return "Recovery timelock must be at least 1 block";
        if (blocks > 10 * 52_560) {
          return "Recovery timelock is too large (max ~10 years)";
        }
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

  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Create vault</h2>
          <p>
            Phase 2 · templates, multi-key, multi recovery · Taproot Miniscript.
          </p>
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
        {step === 1 && (
          <div className="form-grid">
            <h3>Step 1 · Template & wallet</h3>
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
            <div className="template-grid">
              {POLICY_TEMPLATES.map((tpl) => (
                <button
                  key={tpl.id}
                  type="button"
                  className={
                    tpl.id === templateId
                      ? "template-card selected"
                      : "template-card"
                  }
                  onClick={() => applyTemplate(tpl)}
                >
                  <strong>{tpl.label}</strong>
                  <span className="muted">{tpl.description}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        {step === 2 && (
          <div className="form-grid">
            <h3>Step 2 · Keys</h3>
            <p className="muted">
              Add investors, managers, or recovery keys. Id must match the
              primary / recovery expressions (A, B, C…).
            </p>
            {keys.map((key, index) => (
              <div key={`${key.id}-${index}`} className="key-block">
                <div className="row-actions">
                  <strong>
                    {template.defaultKeys[index]?.label ?? `Key ${key.id}`}
                  </strong>
                  {keys.length > 1 ? (
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => removeKey(index)}
                    >
                      Remove
                    </button>
                  ) : null}
                </div>
                <label>
                  Id
                  <input
                    value={key.id}
                    onChange={(e) =>
                      updateKey(index, { id: e.target.value.trim() })
                    }
                    required
                  />
                </label>
                <label>
                  Role
                  <select
                    value={key.role}
                    onChange={(e) =>
                      updateKey(index, {
                        role: e.target.value as KeyRole,
                      })
                    }
                  >
                    {ROLE_OPTIONS.map((role) => (
                      <option key={role} value={role}>
                        {role}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  XPUB
                  <textarea
                    rows={2}
                    value={key.xpub}
                    onChange={(e) =>
                      updateKey(index, { xpub: e.target.value.trim() })
                    }
                    placeholder="xpub… or tpub…"
                    required
                  />
                </label>
                <label>
                  Fingerprint (8 hex)
                  <input
                    value={key.fingerprint}
                    onChange={(e) =>
                      updateKey(index, {
                        fingerprint: e.target.value.trim(),
                      })
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
                      updateKey(index, {
                        origin_path: e.target.value.trim(),
                      })
                    }
                    placeholder="86'/1'/0'"
                  />
                </label>
              </div>
            ))}
            <div className="row-actions">
              <button
                type="button"
                className="secondary"
                onClick={() => addKey("manager")}
              >
                Add manager
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => addKey("recovery")}
              >
                Add recovery key
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() => addKey("investor")}
              >
                Add investor
              </button>
            </div>
          </div>
        )}

        {step === 3 && (
          <div className="form-grid">
            <h3>Step 3 · Primary expression</h3>
            <p className="muted">
              Miniscript Builder (basic): use key ids with{" "}
              <code>&&</code> / <code>||</code> and parentheses. Available keys:{" "}
              {keys.map((k) => k.id).join(", ") || "—"}.
            </p>
            <label>
              Primary policy
              <textarea
                rows={3}
                value={primary}
                onChange={(e) => setPrimary(e.target.value)}
                className="mono"
                required
              />
            </label>
            <div className="row-actions">
              <button
                type="button"
                className="secondary"
                onClick={() => setPrimary(template.defaultPrimary)}
              >
                Reset to template
              </button>
              {templateId === "multi_manager" ? (
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    setPrimary(multiManagerPrimary(keys.map((k) => k.id)))
                  }
                >
                  Rebuild A∧manager chain
                </button>
              ) : null}
            </div>
            <div className="builder-chips">
              {keys.map((k) => (
                <button
                  key={k.id}
                  type="button"
                  className="secondary"
                  onClick={() =>
                    setPrimary((prev) =>
                      prev.trim() ? `${prev.trim()} && ${k.id}` : k.id,
                    )
                  }
                >
                  + {k.id}
                </button>
              ))}
              <button
                type="button"
                className="secondary"
                onClick={() => setPrimary((p) => `(${p.trim() || "A && B"})`)}
              >
                ( group )
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() =>
                  setPrimary((p) => (p.trim() ? `${p.trim()} || ` : ""))
                }
              >
                ||
              </button>
              <button
                type="button"
                className="secondary"
                onClick={() =>
                  setPrimary((p) => (p.trim() ? `${p.trim()} && ` : ""))
                }
              >
                &&
              </button>
            </div>
          </div>
        )}

        {step === 4 && (
          <div className="form-grid">
            <h3>Step 4 · Recovery / inheritance paths</h3>
            <p className="muted">
              Each path becomes a Taproot leaf{" "}
              <code>and(older(N), …)</code>. Leave empty for no timelock
              fallback. <code>allow</code> may be a key or expression (e.g.{" "}
              <code>A && B</code>).
            </p>
            {recoveryPaths.length === 0 ? (
              <p className="muted">No recovery paths.</p>
            ) : null}
            {recoveryPaths.map((path, index) => (
              <div key={index} className="key-block">
                <div className="row-actions">
                  <strong>Path {index + 1}</strong>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() =>
                      setRecoveryPaths((prev) =>
                        prev.filter((_, i) => i !== index),
                      )
                    }
                  >
                    Remove
                  </button>
                </div>
                <label>
                  Amount
                  <input
                    type="number"
                    min={1}
                    value={path.amount}
                    onChange={(e) =>
                      setRecoveryPaths((prev) =>
                        prev.map((p, i) =>
                          i === index
                            ? { ...p, amount: Number(e.target.value) }
                            : p,
                        ),
                      )
                    }
                  />
                </label>
                <label>
                  Unit
                  <select
                    value={path.unit}
                    onChange={(e) =>
                      setRecoveryPaths((prev) =>
                        prev.map((p, i) =>
                          i === index
                            ? {
                                ...p,
                                unit: e.target.value as TimelockUnit,
                              }
                            : p,
                        ),
                      )
                    }
                  >
                    <option value="d">Days</option>
                    <option value="w">Weeks</option>
                    <option value="y">Years</option>
                    <option value="b">Blocks</option>
                  </select>
                </label>
                <label>
                  Allow (key / expression)
                  <input
                    value={path.allow}
                    onChange={(e) =>
                      setRecoveryPaths((prev) =>
                        prev.map((p, i) =>
                          i === index
                            ? { ...p, allow: e.target.value }
                            : p,
                        ),
                      )
                    }
                    placeholder="A or A && B"
                  />
                </label>
                <p className="mono muted">
                  {durationToBlocks(path.amount, path.unit).toLocaleString()}{" "}
                  blocks
                </p>
              </div>
            ))}
            <button
              type="button"
              className="secondary"
              onClick={() =>
                setRecoveryPaths((prev) => [
                  ...prev,
                  {
                    amount: 1,
                    unit: "y",
                    allow: keys.find((k) => k.id !== "A")?.id ?? "B",
                  },
                ])
              }
            >
              Add recovery path
            </button>
          </div>
        )}

        {step === 5 && (
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
            <p className="muted">
              Template: <strong>{template.label}</strong>
            </p>
            <pre className="mono review">{JSON.stringify(policy, null, 2)}</pre>
            <div className="row-actions">
              <button
                type="button"
                className="secondary"
                onClick={() => void onPreview()}
              >
                Preview descriptor
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
          {step < 5 ? (
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
          ) : null}
        </div>
      </form>

      {error ? <pre className="error">{error}</pre> : null}
    </section>
  );
}

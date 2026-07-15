import { Link } from "react-router-dom";
import type { HotWalletSummaryDto, WalletDto } from "../lib/types";

export type SignMethod = "hot" | "software" | "hardware" | "combine";

const METHODS: { id: SignMethod; label: string; hint: string }[] = [
  {
    id: "hot",
    label: "Hot wallet",
    hint: "Unlocked keystore on this machine",
  },
  {
    id: "software",
    label: "Software key",
    hint: "Paste tprv / xprv with derivation path",
  },
  {
    id: "hardware",
    label: "Hardware (HWI)",
    hint: "Coldcard, Ledger, Trezor via HWI",
  },
  {
    id: "combine",
    label: "Combine cosigner PSBT",
    hint: "Merge a partially signed PSBT from another machine",
  },
];

type Props = {
  method: SignMethod;
  onMethodChange: (method: SignMethod) => void;
  wallet: WalletDto | null;
  busy: boolean;
  /** Which action just succeeded — drives green pulse on that button. */
  successMethod?: SignMethod | null;
  hotWallets: HotWalletSummaryDto[];
  hotWalletId: string;
  onHotWalletIdChange: (id: string) => void;
  secretKey: string;
  onSecretKeyChange: (value: string) => void;
  hwFingerprint: string;
  onHwFingerprintChange: (value: string) => void;
  cosignerPsbt: string;
  onCosignerPsbtChange: (value: string) => void;
  allowMainnetHotKeys: boolean;
  onAllowMainnetHotKeysChange: (value: boolean) => void;
  confirmMainnetHot: boolean;
  onConfirmMainnetHotChange: (value: boolean) => void;
  onSignHot: () => void;
  onSignSoftware: () => void;
  onSignHardware: () => void;
  onCombine: () => void;
};

function actionLabel(ok: boolean, idle: string, done: string): string {
  return ok ? done : idle;
}

export function PsbtSignMethodPanel({
  method,
  onMethodChange,
  wallet,
  busy,
  successMethod = null,
  hotWallets,
  hotWalletId,
  onHotWalletIdChange,
  secretKey,
  onSecretKeyChange,
  hwFingerprint,
  onHwFingerprintChange,
  cosignerPsbt,
  onCosignerPsbtChange,
  allowMainnetHotKeys,
  onAllowMainnetHotKeysChange,
  confirmMainnetHot,
  onConfirmMainnetHotChange,
  onSignHot,
  onSignSoftware,
  onSignHardware,
  onCombine,
}: Props) {
  const mainnet = wallet?.policy.network === "mainnet";
  const ok = (m: SignMethod) => successMethod === m;

  return (
    <div className="form-grid">
      <label>
        Signed with
        <select
          value={method}
          onChange={(e) => onMethodChange(e.target.value as SignMethod)}
        >
          {METHODS.map((m) => (
            <option key={m.id} value={m.id}>
              {m.label}
            </option>
          ))}
        </select>
      </label>
      <p className="muted">{METHODS.find((m) => m.id === method)?.hint}</p>

      {method === "hot" ? (
        <>
          <label>
            Hot wallet
            <select
              value={hotWalletId}
              onChange={(e) => onHotWalletIdChange(e.target.value)}
            >
              <option value="">— select —</option>
              {hotWallets.map((hw) => (
                <option key={hw.id} value={hw.id}>
                  {hw.name} · {hw.fingerprint}
                </option>
              ))}
            </select>
          </label>
          {hotWallets.length === 0 ? (
            <p className="muted">
              Unlock / import under <Link to="/hot-wallets">Hot wallets</Link>.
            </p>
          ) : null}
          {mainnet ? (
            <MainnetHotConfirm
              allowMainnetHotKeys={allowMainnetHotKeys}
              confirmMainnetHot={confirmMainnetHot}
              onAllowMainnetHotKeysChange={onAllowMainnetHotKeysChange}
              onConfirmMainnetHotChange={onConfirmMainnetHotChange}
            />
          ) : null}
          <button
            type="button"
            className={ok("hot") ? "btn-ok" : undefined}
            disabled={busy || !hotWalletId}
            onClick={onSignHot}
          >
            {actionLabel(ok("hot"), "Sign with hot wallet", "Signed ✓")}
          </button>
        </>
      ) : null}

      {method === "software" ? (
        <>
          <label>
            Descriptor secret (tprv/xprv… with path)
            <textarea
              rows={2}
              className="mono"
              value={secretKey}
              onChange={(e) => onSecretKeyChange(e.target.value)}
              placeholder="tprv…/86'/1'/0'/0/*"
              autoComplete="off"
            />
          </label>
          {mainnet ? (
            <MainnetHotConfirm
              allowMainnetHotKeys={allowMainnetHotKeys}
              confirmMainnetHot={confirmMainnetHot}
              onAllowMainnetHotKeysChange={onAllowMainnetHotKeysChange}
              onConfirmMainnetHotChange={onConfirmMainnetHotChange}
            />
          ) : null}
          <button
            type="button"
            className={ok("software") ? "btn-ok" : undefined}
            disabled={busy || !secretKey.trim()}
            onClick={onSignSoftware}
          >
            {actionLabel(ok("software"), "Sign with software key", "Signed ✓")}
          </button>
        </>
      ) : null}

      {method === "hardware" ? (
        <>
          <label>
            Hardware fingerprint (HWI)
            <input
              className="mono"
              value={hwFingerprint}
              onChange={(e) => onHwFingerprintChange(e.target.value)}
              placeholder="Settings → Signing devices"
            />
          </label>
          <button
            type="button"
            className={ok("hardware") ? "btn-ok" : undefined}
            disabled={busy || !hwFingerprint.trim()}
            onClick={onSignHardware}
          >
            {actionLabel(ok("hardware"), "Sign with hardware", "Signed ✓")}
          </button>
        </>
      ) : null}

      {method === "combine" ? (
        <>
          <label>
            Cosigner PSBT (base64)
            <textarea
              rows={4}
              className="mono"
              value={cosignerPsbt}
              onChange={(e) => onCosignerPsbtChange(e.target.value)}
              placeholder="Paste partially signed PSBT from another signer"
            />
          </label>
          <button
            type="button"
            className={ok("combine") ? "btn-ok" : undefined}
            disabled={busy || !cosignerPsbt.trim()}
            onClick={onCombine}
          >
            {actionLabel(
              ok("combine"),
              "Combine with cosigner PSBT",
              "Combined ✓",
            )}
          </button>
        </>
      ) : null}
    </div>
  );
}

function MainnetHotConfirm({
  allowMainnetHotKeys,
  confirmMainnetHot,
  onAllowMainnetHotKeysChange,
  onConfirmMainnetHotChange,
}: {
  allowMainnetHotKeys: boolean;
  confirmMainnetHot: boolean;
  onAllowMainnetHotKeysChange: (value: boolean) => void;
  onConfirmMainnetHotChange: (value: boolean) => void;
}) {
  return (
    <>
      <label className="check-row">
        <input
          type="checkbox"
          checked={allowMainnetHotKeys}
          onChange={(e) => onAllowMainnetHotKeysChange(e.target.checked)}
        />
        <span>Allow mainnet hot-key signing (dangerous)</span>
      </label>
      <label className="check-row">
        <input
          type="checkbox"
          checked={confirmMainnetHot}
          onChange={(e) => onConfirmMainnetHotChange(e.target.checked)}
        />
        <span>
          I understand this exposes private key material on this machine
        </span>
      </label>
    </>
  );
}

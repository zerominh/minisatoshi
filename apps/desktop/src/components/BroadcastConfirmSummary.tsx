import { useT } from "../i18n/LocaleContext";
import { formatBtcFromSats, formatNetwork, formatSats } from "../lib/settings";
import type { NetworkName, TxOutputDto } from "../lib/types";

type Props = {
  fromWallet: string;
  network: NetworkName | null | undefined;
  /** Primary recipient (compose form). */
  toAddress?: string | null;
  /** Primary amount in sats (compose form). */
  amountSats?: number | null;
  /** All tx outputs after finalize (includes change). */
  outputs?: TxOutputDto[] | null;
};

export function BroadcastConfirmSummary({
  fromWallet,
  network,
  toAddress,
  amountSats,
  outputs,
}: Props) {
  const t = useT();
  const amount =
    amountSats != null && Number.isFinite(amountSats) && amountSats > 0
      ? amountSats
      : null;
  const to = toAddress?.trim() || null;
  const hasCompose = Boolean(to && amount != null);

  const extraOutputs =
    outputs?.filter((o) => {
      if (!to) return false;
      return (o.address ?? "") !== to;
    }) ?? [];

  return (
    <div className="broadcast-confirm">
      <h4>{t("send.confirmTitle")}</h4>
      <dl className="broadcast-confirm-grid">
        <dt>{t("send.confirmFrom")}</dt>
        <dd>
          <strong>{fromWallet}</strong>
        </dd>
        {hasCompose ? (
          <>
            <dt>{t("send.confirmTo")}</dt>
            <dd className="mono wrap">{to}</dd>
            <dt>{t("send.confirmAmount")}</dt>
            <dd>
              <strong className="broadcast-confirm-amount">
                {formatSats(amount!)}
              </strong>
              <span className="muted"> · {formatBtcFromSats(amount!)}</span>
            </dd>
          </>
        ) : null}
        <dt>{t("send.confirmNetwork")}</dt>
        <dd>
          <strong>{network ? formatNetwork(network) : "—"}</strong>
        </dd>
      </dl>
      {!hasCompose && outputs && outputs.length > 0 ? (
        <div className="broadcast-confirm-outputs">
          <p className="muted">{t("send.confirmTo")}</p>
          <ul>
            {outputs.map((o, i) => (
              <li key={`${o.address ?? "out"}-${i}`}>
                <span className="mono wrap">{o.address ?? "(non-standard)"}</span>
                <strong className="broadcast-confirm-amount">
                  {formatSats(o.amountSats)}
                </strong>
                <span className="muted">
                  {" "}
                  · {formatBtcFromSats(o.amountSats)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      {hasCompose && extraOutputs.length > 0 ? (
        <div className="broadcast-confirm-outputs">
          <p className="muted">Other outputs (e.g. change):</p>
          <ul>
            {extraOutputs.map((o, i) => (
              <li key={`${o.address ?? "out"}-${i}`}>
                <span className="mono wrap">{o.address ?? "(non-standard)"}</span>
                <span>
                  {formatSats(o.amountSats)} · {formatBtcFromSats(o.amountSats)}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
      <p className="broadcast-confirm-hint">{t("send.confirmHint")}</p>
    </div>
  );
}

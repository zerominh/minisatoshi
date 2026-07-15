import { Link } from "react-router-dom";
import { useT } from "../i18n/LocaleContext";

export function TransactionsPage() {
  const t = useT();
  return (
    <section>
      <header className="page-header">
        <div>
          <h2>{t("tx.globalTitle")}</h2>
          <p>{t("tx.globalHint")}</p>
        </div>
      </header>
      <div className="panel">
        <p className="muted">{t("tx.globalHint")}</p>
        <Link className="button-link" to="/wallets">
          {t("nav.wallets")}
        </Link>
      </div>
    </section>
  );
}

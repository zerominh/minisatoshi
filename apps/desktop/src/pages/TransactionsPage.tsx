import { Link } from "react-router-dom";

export function TransactionsPage() {
  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Transactions</h2>
          <p>
            History lives on each vault (Transactions tab), like Sparrow&apos;s
            per-wallet view. Global aggregation comes later.
          </p>
        </div>
      </header>
      <div className="panel">
        <p className="muted">
          Open a vault → <strong>Transactions</strong>, sync, then review history.
        </p>
        <Link className="button-link" to="/vaults">
          Browse vaults
        </Link>
      </div>
    </section>
  );
}

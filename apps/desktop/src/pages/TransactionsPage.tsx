import { Link } from "react-router-dom";

export function TransactionsPage() {
  return (
    <section>
      <header className="page-header">
        <div>
          <h2>Transactions</h2>
          <p>
            Per-vault history lives on each vault dashboard after sync. Global
            history aggregation comes in a later sprint.
          </p>
        </div>
      </header>
      <div className="panel">
        <p className="muted">
          Open a vault, tap <strong>Sync chain</strong>, then review Recent TX.
        </p>
        <Link className="button-link" to="/vaults">
          Browse vaults
        </Link>
      </div>
    </section>
  );
}

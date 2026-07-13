function App() {
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
        <p>Offline desktop app for creating and managing Bitcoin vaults with Miniscript.</p>
        <p className="status">Sprint 0 complete — policy engine ready.</p>
      </main>
    </div>
  );
}

export default App;

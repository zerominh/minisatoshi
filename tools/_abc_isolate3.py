#!/usr/bin/env python3
"""Test taproot script-path syntax: braces vs bare leaf."""
from __future__ import annotations

import json

from ledger_bitcoin import WalletPolicy, createClient, Chain


def try_register(name: str, policy: str, keys: list[str]) -> None:
    print(f"\n=== {name} ===", flush=True)
    print(f"policy={policy}", flush=True)
    client = createClient(chain=Chain.TEST)
    try:
        wallet = WalletPolicy(name[:16], policy, keys)
        _id, hmac = client.register_wallet(wallet)
        print(json.dumps({"ok": True, "hmac": hmac.hex()}), flush=True)
    except Exception as exc:  # noqa: BLE001
        print(json.dumps({"ok": False, "error": str(exc)}), flush=True)
    finally:
        client.stop()


def main() -> None:
    client = createClient(chain=Chain.TEST)
    try:
        fp = client.get_master_fingerprint().hex()
        t0 = client.get_extended_pubkey("86'/1'/0'", False)
        t1 = client.get_extended_pubkey("86'/1'/1'", False)
        t2 = client.get_extended_pubkey("84'/1'/0'", False)
        t3 = client.get_extended_pubkey("84'/1'/1'", False)
    finally:
        client.stop()

    k0 = f"[{fp}/86'/1'/0']{t0}"
    k1 = f"[{fp}/86'/1'/1']{t1}"
    k2 = f"[{fp}/84'/1'/0']{t2}"
    k3 = f"[{fp}/84'/1'/1']{t3}"
    print(f"device fp={fp}", flush=True)

    # Single leaf WITHOUT braces (BIP-386 / wallet policy TREE = SCRIPT)
    try_register("bare pk leaf", "tr(@0/**,pk(@1/**))", [k0, k1])

    # Single leaf with braces (invalid TREE — brace form needs TWO children)
    try_register("braced pk leaf", "tr(@0/**,{pk(@1/**)})", [k0, k1])

    # Two-leaf tree (braces required)
    try_register(
        "two pk leaves",
        "tr(@0/**,{pk(@1/**),pk(@2/**)})",
        [k0, k1, k2],
    )

    # and_v bare (single leaf)
    try_register(
        "bare and_v",
        "tr(@0/**,and_v(v:pk(@1/**),pk(@2/**)))",
        [k0, k1, k2],
    )

    # multi_a bare
    try_register(
        "bare multi_a",
        "tr(@0/**,multi_a(2,@1/**,@2/**))",
        [k0, k1, k2],
    )

    # ABC-shaped two and_v leaves
    try_register(
        "two and_v",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**)),and_v(v:pk(@1/**),pk(@3/**))})",
        [k0, k1, k2, k3],
    )


if __name__ == "__main__":
    main()

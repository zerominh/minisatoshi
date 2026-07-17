#!/usr/bin/env python3
"""Confirm multi-leaf and_v limitation vs mixed trees / single-leaf or_."""
from __future__ import annotations

import json

from ledger_bitcoin import WalletPolicy, createClient, Chain
from ledger_bitcoin.embit.bip32 import HDKey
from ledger_bitcoin.embit.ec import NUMS_PUBKEY
from ledger_bitcoin.embit.networks import NETWORKS


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
        keys_raw = [
            client.get_extended_pubkey("86'/1'/0'", False),
            client.get_extended_pubkey("86'/1'/1'", False),
            client.get_extended_pubkey("84'/1'/0'", False),
            client.get_extended_pubkey("84'/1'/1'", False),
        ]
    finally:
        client.stop()

    k = [
        f"[{fp}/86'/1'/0']{keys_raw[0]}",
        f"[{fp}/86'/1'/1']{keys_raw[1]}",
        f"[{fp}/84'/1'/0']{keys_raw[2]}",
        f"[{fp}/84'/1'/1']{keys_raw[3]}",
    ]
    dummy = HDKey(
        NUMS_PUBKEY, bytes(32), version=NETWORKS["test"]["xpub"]
    ).to_string()
    dummy_key = f"[50929b74/86'/1'/0']{dummy}"
    print(f"device fp={fp}", flush=True)

    # Mixed tree: and_v + pk
    try_register(
        "and_v + pk",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**)),pk(@3/**)})",
        k,
    )

    # Two and_v again (known fail) — control
    try_register(
        "two and_v",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**)),and_v(v:pk(@1/**),pk(@3/**))})",
        k,
    )

    # Single-leaf or_i of two and_v (different address than taproot tree; syntax probe)
    try_register(
        "or_i of and_v",
        "tr(@0/**,or_i(and_v(v:pk(@1/**),pk(@2/**)),and_v(v:pk(@1/**),pk(@3/**))))",
        k,
    )

    # NUMS + bare and_v (Minisatoshi-like single primary path)
    try_register(
        "NUMS+and_v",
        "tr(@0/**,and_v(v:pk(@1/**),pk(@2/**)))",
        [dummy_key, k[1], k[2]],
    )

    # NUMS + two pk leaves (no and_v)
    try_register(
        "NUMS+two pk",
        "tr(@0/**,{pk(@1/**),pk(@2/**)})",
        [dummy_key, k[1], k[2]],
    )


if __name__ == "__main__":
    main()

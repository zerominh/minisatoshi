#!/usr/bin/env python3
"""Retest taproot script trees using ONLY keys from this Ledger (correct paths)."""
from __future__ import annotations

import json

from ledger_bitcoin import WalletPolicy, createClient, Chain
from ledger_bitcoin.embit.bip32 import HDKey
from ledger_bitcoin.embit.ec import NUMS_PUBKEY
from ledger_bitcoin.embit.networks import NETWORKS


def try_register(name: str, policy: str, keys: list[str]) -> None:
    print(f"\n=== {name} ===", flush=True)
    print(f"policy={policy}", flush=True)
    for i, k in enumerate(keys):
        print(f"  key[{i}]={k[:72]}...", flush=True)
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
        # Two distinct account xpubs from THIS device, correct origins.
        tpub86_0 = client.get_extended_pubkey("86'/1'/0'", False)
        tpub86_1 = client.get_extended_pubkey("86'/1'/1'", False)
        tpub84_0 = client.get_extended_pubkey("84'/1'/0'", False)
    finally:
        client.stop()

    k86_0 = f"[{fp}/86'/1'/0']{tpub86_0}"
    k86_1 = f"[{fp}/86'/1'/1']{tpub86_1}"
    k84_0 = f"[{fp}/84'/1'/0']{tpub84_0}"

    dummy_xpub = HDKey(
        NUMS_PUBKEY, bytes(32), version=NETWORKS["test"]["xpub"]
    ).to_string()
    # Depth-0 NUMS xpub — try with and without a claimed BIP path.
    dummy_bare = dummy_xpub  # no origin (external)
    dummy_fp = f"[50929b74]{dummy_xpub}"
    dummy_path = f"[50929b74/86'/1'/0']{dummy_xpub}"

    print(f"device fp={fp}", flush=True)

    # Control (already known OK)
    try_register("BIP86", "tr(@0/**)", [k86_0])

    # Script tree, both keys from device — if this works, prior 0x6a80 was bad external tpub/path
    try_register("tr+pk both device", "tr(@0/**,{pk(@1/**)})", [k86_0, k86_1])

    try_register(
        "tr+and_v both device",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**))})",
        [k86_0, k86_1, k84_0],
    )

    try_register("tr+multi_a", "tr(@0/**,{multi_a(2,@1/**,@2/**)})", [k86_0, k86_1, k84_0])

    # NUMS variants + device script key
    try_register("NUMS bare+pk", "tr(@0/**,{pk(@1/**)})", [dummy_fp, k86_0])
    try_register("NUMS path+pk", "tr(@0/**,{pk(@1/**)})", [dummy_path, k86_0])
    try_register("NUMS no brackets+pk", "tr(@0/**,{pk(@1/**)})", [dummy_bare, k86_0])


if __name__ == "__main__":
    main()

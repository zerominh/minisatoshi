#!/usr/bin/env python3
"""Isolate which ABC taproot piece Bitcoin Test rejects (0x6a82)."""
from __future__ import annotations

import json
import sys

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
        tpub86 = client.get_extended_pubkey("86'/1'/0'", False)
        tpub84 = client.get_extended_pubkey("84'/1'/0'", False)
    finally:
        client.stop()

    our86 = f"[{fp}/86'/1'/0']{tpub86}"
    our84 = f"[{fp}/84'/1'/0']{tpub84}"
    ext = (
        "[76223a6e/86'/1'/0']"
        "tpubDE7NQymr4AFtewpAsWtnreyq9ghkzQBXpCZjWLFVRAvnbf7vya2eMTvT2fPapNqL8SuVvLQdbUbMfWLVDCZKnsEBqp6UK93QEzL8Ck23AwF"
    )
    dummy_xpub = HDKey(
        NUMS_PUBKEY, bytes(32), version=NETWORKS["test"]["xpub"]
    ).to_string()
    dummy_pathed = f"[50929b74/86'/1'/0']{dummy_xpub}"
    dummy_bare = f"[50929b74]{dummy_xpub}"

    print(f"device fp={fp}", flush=True)

    # 1) BIP-86 single key (usually no register needed; still try)
    try_register("tr BIP86", "tr(@0/**)", [our86])

    # 2) Taproot script: pk only, device internal (no NUMS)
    try_register(
        "tr+pk leaf",
        "tr(@0/**,{pk(@1/**)})",
        [our86, ext],
    )

    # 3) Taproot and_v, device internal (no NUMS)
    try_register(
        "tr+and_v no NUMS",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**))})",
        [our86, our84, ext],
    )

    # 4) NUMS dummy + pk leaf
    try_register(
        "NUMS+pk",
        "tr(@0/**,{pk(@1/**)})",
        [dummy_pathed, our86],
    )

    # 5) NUMS bare origin + and_v (ABC-like)
    try_register(
        "NUMS bare+and_v",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**))})",
        [dummy_bare, our86, ext],
    )

    # 6) Full ABC-shaped no older (current smoke)
    try_register(
        "ABC no older",
        "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**)),and_v(v:pk(@1/**),pk(@3/**))})",
        [dummy_pathed, our86, ext, our84],
    )


if __name__ == "__main__":
    main()

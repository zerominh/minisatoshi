#!/usr/bin/env python3
"""Minisatoshi ↔ ledger-bitcoin bridge (stdin/stdout JSON).

Commands:
  register  Read JSON from stdin, register WalletPolicy on Ledger, print hmac hex.
  sign      Read JSON from stdin, sign PSBT, print base64 PSBT with signatures applied.

Requires: pip install ledger-bitcoin
"""

from __future__ import annotations

import argparse
import base64
import json
import multiprocessing
import sys
from typing import Any

from ledger_bitcoin import WalletPolicy, createClient, Chain, Client, TransportClient
from ledger_bitcoin.client_base import PartialSignature, MusigPubNonce, MusigPartialSignature
from ledger_bitcoin.psbt import PSBT

# HID open / APDU can block forever if Ledger Live holds the device or no app is open.
PROBE_TIMEOUT_SECS = 12


CHAIN_MAP = {
    "main": Chain.MAIN,
    "mainnet": Chain.MAIN,
    "test": Chain.TEST,
    "testnet": Chain.TEST,
    "testnet3": Chain.TEST,
    "testnet4": Chain.TEST,
    "regtest": Chain.REGTEST,
    "signet": Chain.SIGNET,
}


def emit(obj: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(obj))
    sys.stdout.flush()


def fail(msg: str, code: int = 1) -> None:
    emit({"ok": False, "error": msg})
    sys.exit(code)


def parse_chain(raw: str) -> Chain:
    key = raw.strip().lower()
    if key not in CHAIN_MAP:
        fail(f"unsupported chain: {raw}")
    return CHAIN_MAP[key]


def wallet_from_payload(payload: dict[str, Any]) -> WalletPolicy:
    name = payload.get("name") or payload.get("policyName") or "Minisatoshi"
    policy = payload.get("policy") or payload.get("policyTemplate")
    keys = payload.get("keys") or payload.get("keysInfo")
    if not policy or not keys:
        fail("missing policy and keys in request")
    if not isinstance(keys, list) or not all(isinstance(k, str) for k in keys):
        fail("keys must be a list of strings")
    return WalletPolicy(str(name), str(policy), keys)


def hmac_from_hex(raw: str) -> bytes:
    cleaned = raw.strip().lower().removeprefix("0x")
    if len(cleaned) != 64:
        fail(f"invalid hmac hex length: {len(cleaned)}")
    try:
        return bytes.fromhex(cleaned)
    except ValueError as exc:
        fail(f"invalid hmac hex: {exc}")


def apply_signatures(psbt: PSBT, results: list) -> None:
    for input_index, obj in results:
        if input_index < 0 or input_index >= len(psbt.inputs):
            continue
        inp = psbt.inputs[input_index]
        if isinstance(obj, PartialSignature):
            if obj.tapleaf_hash is not None:
                inp.tap_script_sigs[(obj.pubkey, obj.tapleaf_hash)] = obj.signature
            else:
                inp.tap_key_sig = obj.signature
        elif isinstance(obj, (MusigPubNonce, MusigPartialSignature)):
            fail("MuSig2 PSBT signing is not supported in Minisatoshi yet")


def _probe_worker(chain_value: int, queue: "multiprocessing.Queue[object]") -> None:
    """Run in a child process so a stuck HID open can be terminated."""
    try:
        chain = Chain(chain_value)  # type: ignore[arg-type]
        comm_client = TransportClient("hid")
        client = Client(comm_client, chain, False)
        try:
            app_name, app_version, _ = client.get_version()
            queue.put({"ok": True, "appName": app_name, "appVersion": app_version})
        finally:
            client.stop()
    except Exception as exc:  # noqa: BLE001
        queue.put({"ok": False, "error": str(exc)})


def cmd_probe(chain: Chain) -> None:
    ctx = multiprocessing.get_context("spawn")
    queue: multiprocessing.Queue = ctx.Queue()
    proc = ctx.Process(target=_probe_worker, args=(chain.value, queue))
    proc.start()
    proc.join(PROBE_TIMEOUT_SECS)
    if proc.is_alive():
        proc.terminate()
        proc.join(2)
        fail(
            f"probe timed out after {PROBE_TIMEOUT_SECS}s. "
            "Unlock Ledger, open Bitcoin Test (or Bitcoin), and close Ledger Live / other wallets using the device."
        )
    if queue.empty():
        fail("probe failed — no response from device (is Bitcoin Test open?)")
    result = queue.get()
    if not result.get("ok"):
        fail(result.get("error") or "probe failed")
    emit(
        {
            "ok": True,
            "appName": result["appName"],
            "appVersion": result["appVersion"],
        }
    )


def cmd_register(chain: Chain, payload: dict[str, Any]) -> None:
    wallet = wallet_from_payload(payload)
    client = createClient(chain=chain)
    try:
        _wallet_id, wallet_hmac = client.register_wallet(wallet)
        emit({"ok": True, "hmac": wallet_hmac.hex()})
    except Exception as exc:  # noqa: BLE001 — surface to Rust
        fail(str(exc))
    finally:
        client.stop()


def _device_key_info(client: Any, path: str) -> tuple[str, str]:
    fp = client.get_master_fingerprint().hex()
    xpub = client.get_extended_pubkey(path, display=False)
    return fp, f"[{fp}/{path}]{xpub}"


def cmd_baseline_register(chain: Chain) -> None:
    """Register a 2-of-2 wsh policy that includes THIS device's key.

    Fixture-only keys (no matching fingerprint) often return 0x6a80/0x6a82 on
    Bitcoin app ≥ 2.4.3 — that is not a client-library bug.
    """
    if chain == Chain.MAIN:
        path = "48'/0'/0'/2'"
        external = (
            "[76223a6e/48'/0'/0'/2']"
            "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL"
        )
    else:
        path = "48'/1'/0'/2'"
        # Cosigner from Ledger Bitcoin app docs (testnet).
        external = (
            "[76223a6e/48'/1'/0'/2']"
            "tpubDE7NQymr4AFtewpAsWtnreyq9ghkzQBXpCZjWLFVRAvnbf7vya2eMTvT2fPapNqL8SuVvLQdbUbMfWLVDCZKnsEBqp6UK93QEzL8Ck23AwF"
        )

    client = createClient(chain=chain)
    try:
        fp, our_key = _device_key_info(client, path)
        wallet = WalletPolicy(
            "Minisatoshi baseline",
            "wsh(sortedmulti(2,@0/**,@1/**))",
            [our_key, external],
        )
        sys.stderr.write(f"fingerprint={fp} path={path}\n")
        sys.stderr.write(f"ourKey={our_key}\n")
        sys.stderr.write("Approve wallet policy on Ledger...\n")
        sys.stderr.flush()
        _wallet_id, wallet_hmac = client.register_wallet(wallet)
        emit({"ok": True, "hmac": wallet_hmac.hex(), "fingerprint": fp})
    except Exception as exc:  # noqa: BLE001
        fail(str(exc))
    finally:
        client.stop()


def cmd_abc_smoke_register(chain: Chain, with_older: bool) -> None:
    """Taproot ABC-shaped policy using THIS device as key A (@1 after NUMS dummy)."""
    from ledger_bitcoin.embit.bip32 import HDKey
    from ledger_bitcoin.embit.ec import NUMS_PUBKEY
    from ledger_bitcoin.embit.networks import NETWORKS

    coin = "0'" if chain == Chain.MAIN else "1'"
    path_a = f"86'/{coin}/0'"
    net = "main" if chain == Chain.MAIN else "test"
    dummy_xpub = HDKey(
        NUMS_PUBKEY, bytes(32), version=NETWORKS[net]["xpub"]
    ).to_string()
    dummy = f"[50929b74/86'/{coin}/0']{dummy_xpub}"

    # External cosigners (B/C) — fixture-style; device is A.
    key_b = (
        f"[73c5da0a/86'/{coin}/0']"
        "tpubDC3pD7UZXnsgh3EBjbtBQiB1FnLask7UHBSunZ1DPK4dCFFZoFRkgxHB8gt42FvLzx1DpxfHWxAsYaY6b643RVcGjDxXxns7wKKYnnfEcbB"
    )
    key_c = (
        f"[73c5da0a/84'/{coin}/0']"
        "tpubDCxX2sYFS5bDkSe5GKKYHjBW7tgyN1R3UchpLJvdbf54ohxeGRtd8MbDUe1cguVHe4vnK68DsuD5MXjxi9EXx16rb9EnNsaF5KT99CinaJz"
    )
    if chain == Chain.MAIN:
        key_b = key_b.replace("tpub", "xpub", 1)
        key_c = key_c.replace("tpub", "xpub", 1)

    if with_older:
        # 65535 = BIP68 max; 210240 is rejected on Bitcoin app ≥ 2.4.6
        policy = (
            "tr(@0/**,{{and_v(v:pk(@1/**),pk(@2/**)),and_v(v:pk(@1/**),pk(@3/**))},"
            "{and_v(v:pk(@1/**),older(65535)),0}})"
        )
        name = "ABC older65535"
    else:
        policy = (
            "tr(@0/**,{and_v(v:pk(@1/**),pk(@2/**)),and_v(v:pk(@1/**),pk(@3/**))})"
        )
        name = "ABC no older"

    client = createClient(chain=chain)
    try:
        fp, key_a = _device_key_info(client, path_a)
        keys = [dummy, key_a, key_b, key_c]
        wallet = WalletPolicy(name, policy, keys)
        sys.stderr.write(f"fingerprint={fp} path={path_a} name={name}\n")
        sys.stderr.write(f"policy={policy}\n")
        sys.stderr.write("Approve wallet policy on Ledger...\n")
        sys.stderr.flush()
        _wallet_id, wallet_hmac = client.register_wallet(wallet)
        emit({"ok": True, "hmac": wallet_hmac.hex(), "fingerprint": fp, "name": name})
    except Exception as exc:  # noqa: BLE001
        fail(str(exc))
    finally:
        client.stop()


def cmd_sign(chain: Chain, payload: dict[str, Any]) -> None:
    psbt_b64 = payload.get("psbt") or payload.get("psbtBase64")
    if not psbt_b64 or not isinstance(psbt_b64, str):
        fail("missing psbt base64 in request")
    hmac_raw = payload.get("hmac") or payload.get("hmacHex")
    if not hmac_raw:
        fail("missing hmac in request — register Ledger policy first")
    wallet = wallet_from_payload(payload)
    wallet_hmac = hmac_from_hex(str(hmac_raw))

    psbt = PSBT()
    psbt.deserialize(psbt_b64.strip())

    client = createClient(chain=chain)
    try:
        results = client.sign_psbt(psbt_b64.strip(), wallet, wallet_hmac)
        apply_signatures(psbt, results)
        signed = psbt.serialize()
        if isinstance(signed, bytes):
            signed_b64 = base64.b64encode(signed).decode("ascii")
        else:
            signed_b64 = signed
        emit({"ok": True, "psbt": signed_b64, "signatureCount": len(results)})
    except Exception as exc:  # noqa: BLE001
        fail(str(exc))
    finally:
        client.stop()


def read_stdin_payload() -> dict[str, Any]:
    """Read JSON from stdin. Avoid blocking forever when stdin is an open TTY."""
    if sys.stdin.isatty():
        return {}
    try:
        raw = sys.stdin.read()
        return json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError as exc:
        fail(f"invalid JSON stdin: {exc}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Minisatoshi ledger-bitcoin bridge")
    parser.add_argument(
        "command",
        choices=["register", "sign", "probe", "baseline", "abc-smoke"],
    )
    parser.add_argument("--chain", default="test", help="main|test|regtest|signet")
    parser.add_argument(
        "--with-older",
        action="store_true",
        help="For abc-smoke: include older(65535) recovery leaf",
    )
    args = parser.parse_args()

    chain = parse_chain(args.chain)
    if args.command == "probe":
        cmd_probe(chain)
        return
    if args.command == "baseline":
        cmd_baseline_register(chain)
        return
    if args.command == "abc-smoke":
        cmd_abc_smoke_register(chain, with_older=args.with_older)
        return

    payload = read_stdin_payload()
    if args.command == "register":
        cmd_register(chain, payload)
    else:
        cmd_sign(chain, payload)


if __name__ == "__main__":
    main()

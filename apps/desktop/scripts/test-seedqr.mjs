import assert from "node:assert/strict";
import { entropyToMnemonic } from "@scure/bip39";
import { wordlist } from "@scure/bip39/wordlists/english.js";
import {
  decodeCompactSeedQr,
  decodeStandardSeedQr,
  parseSeedQrPayload,
} from "../src/lib/seedQr.ts";

const digits12 = "073318950739065415961602009907670428187212261116";
const expected12 =
  "forum undo fragile fade shy sign arrest garment culture tube off merit";
assert.equal(decodeStandardSeedQr(digits12), expected12);

const digits24 =
  "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643";
const expected24 =
  "attack pizza motion avocado network gather crop fresh patrol unusual wild holiday candy pony ranch winter theme error hybrid van cereal salon goddess expire";
assert.equal(decodeStandardSeedQr(digits24), expected24);

const entropy = Uint8Array.from([
  0x5b, 0xbd, 0x9d, 0x71, 0xa8, 0xec, 0x79, 0x90, 0x83, 0x1a, 0xff, 0x35, 0x9d,
  0x42, 0x65, 0x45,
]);
assert.equal(decodeCompactSeedQr(entropy), expected12);
assert.equal(entropyToMnemonic(entropy, wordlist), expected12);

assert.equal(parseSeedQrPayload(digits12).format, "standard-seedqr");
assert.equal(parseSeedQrPayload(expected12).format, "plain-text");
assert.equal(
  parseSeedQrPayload(JSON.stringify({ mnemonic: expected12 })).format,
  "json",
);

console.log("seedQr tests ok");

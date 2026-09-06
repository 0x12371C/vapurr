import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const src = fs.readFileSync(path.join(dir, "VapurrForwarder.sol"), "utf8");
const input = {
  language: "Solidity",
  sources: { "VapurrForwarder.sol": { content: src } },
  settings: {
    optimizer: { enabled: true, runs: 200 },
    evmVersion: "paris",
    // Needed: executeBatch's eight dynamic-array parameters plus its
    // locals hit the legacy codegen's 16-slot stack limit otherwise.
    viaIR: true,
    outputSelection: { "*": { "*": ["abi", "evm.bytecode.object"] } },
  },
};
const out = JSON.parse(solc.compile(JSON.stringify(input)));
if (out.errors) {
  for (const e of out.errors) {
    if (e.severity === "error") {
      console.error(e.formattedMessage || e.message);
      process.exit(1);
    }
  }
}
const c = out.contracts["VapurrForwarder.sol"].VapurrForwarder;
const hex = c.evm.bytecode.object;
if (!hex || hex.length < 200) {
  console.error("empty bytecode");
  process.exit(1);
}
const dest = path.join(dir, "..", "crates", "vapurr-relay", "src", "forwarder.hex");
fs.writeFileSync(dest, hex + "\n");
fs.writeFileSync(path.join(dir, "..", "crates", "vapurr-relay", "src", "forwarder.abi.json"), JSON.stringify(c.abi, null, 2) + "\n");
console.log("wrote", dest, "bytes", hex.length / 2);

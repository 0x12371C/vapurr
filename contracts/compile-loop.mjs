import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const src = fs.readFileSync(path.join(dir, "PusdLoop.sol"), "utf8");
const input = {
  language: "Solidity",
  sources: { "PusdLoop.sol": { content: src } },
  settings: {
    optimizer: { enabled: true, runs: 200 },
    evmVersion: "paris",
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
const c = out.contracts["PusdLoop.sol"].PusdLoop;
const hex = c.evm.bytecode.object;
if (!hex || hex.length < 200) {
  console.error("empty bytecode");
  process.exit(1);
}
const dest = path.join(dir, "..", "crates", "vapurr-econ", "src", "loop.hex");
fs.writeFileSync(dest, hex + "\n");
console.log("wrote", dest, "bytes", hex.length / 2);
if (out.errors) {
  for (const e of out.errors) {
    if (e.severity === "warning") console.warn(e.formattedMessage || e.message);
  }
}

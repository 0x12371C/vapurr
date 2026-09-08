import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const src = fs.readFileSync(path.join(dir, "GvFed.sol"), "utf8");
const input = {
  language: "Solidity",
  sources: { "GvFed.sol": { content: src } },
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
const destDir = path.join(dir, "out");
fs.mkdirSync(destDir, { recursive: true });
for (const name of Object.keys(out.contracts["GvFed.sol"])) {
  const c = out.contracts["GvFed.sol"][name];
  const hex = c.evm.bytecode.object;
  if (!hex) continue;
  fs.writeFileSync(path.join(destDir, `GvFed_${name}.bin`), hex + "\n");
  fs.writeFileSync(path.join(destDir, `GvFed_${name}.abi`), JSON.stringify(c.abi, null, 2));
  console.log("wrote", name, "bytes", hex.length / 2);
}

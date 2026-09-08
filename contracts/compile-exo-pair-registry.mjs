import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const src = fs.readFileSync(path.join(dir, "ExogenousPairRegistry.sol"), "utf8");
const input = {
  language: "Solidity",
  sources: { "ExogenousPairRegistry.sol": { content: src } },
  settings: {
    optimizer: { enabled: true, runs: 200 },
    evmVersion: "paris",
    outputSelection: { "*": { "*": ["abi"] } },
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
const c = out.contracts["ExogenousPairRegistry.sol"].ExogenousPairRegistry;
if (!c || !c.abi) {
  console.error("missing ExogenousPairRegistry abi");
  process.exit(1);
}
const dest = path.join(dir, "..", "crates", "vapurr-econ", "src", "exogenous_pair_registry.abi.json");
fs.writeFileSync(dest, JSON.stringify(c.abi, null, 2) + "\n");
console.log("wrote", dest, "funcs", c.abi.filter((x) => x.type === "function").length);

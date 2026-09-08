import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const sourceNames = [
  "IVapurrMinter.sol",
  "GvFed.sol",
  "Remittance.sol",
  "PusdMarket.sol",
  "PusdMarketFed.sol",
  "PusdLoop.sol",
  "LegacyVConverter.sol",
  "LitheCutoverMigrator.sol",
  "CanonicalLitheFactory.sol",
];
const sources = Object.fromEntries(
  sourceNames.map((name) => [name, { content: fs.readFileSync(path.join(dir, name), "utf8") }]),
);
const input = {
  language: "Solidity",
  sources,
  settings: {
    // The factory embeds each successor creation bytecode. Optimize for the
    // EVM initcode ceiling rather than repeat-call gas; the factory is deployed once.
    optimizer: { enabled: true, runs: 1 },
    viaIR: true,
    evmVersion: "paris",
    outputSelection: { "*": { "*": ["evm.bytecode.object"] } },
  },
};
const out = JSON.parse(solc.compile(JSON.stringify(input)));
if (out.errors) {
  for (const error of out.errors) {
    if (error.severity === "error") {
      console.error(error.formattedMessage || error.message);
      process.exit(1);
    }
  }
}
const hex = out.contracts["CanonicalLitheFactory.sol"].CanonicalLitheFactory.evm.bytecode.object;
if (!hex || hex.length < 200) {
  console.error("empty canonical Lithe factory bytecode");
  process.exit(1);
}
const dest = path.join(dir, "..", "crates", "vapurr-econ", "src", "canonical_lithe_factory.hex");
fs.writeFileSync(dest, hex + "\n");
console.log("wrote", dest, "bytes", hex.length / 2);

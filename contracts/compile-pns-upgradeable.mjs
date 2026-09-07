import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const pnsSrc = fs.readFileSync(path.join(dir, "PnsRegistryUpgradeable.sol"), "utf8");
const initSrc = fs.readFileSync(path.join(dir, "proxy", "Initializable.sol"), "utf8");
const uupsSrc = fs.readFileSync(path.join(dir, "proxy", "UUPSUpgradeable.sol"), "utf8");

const input = {
  language: "Solidity",
  sources: {
    "PnsRegistryUpgradeable.sol": { content: pnsSrc },
    "proxy/Initializable.sol": { content: initSrc },
    "proxy/UUPSUpgradeable.sol": { content: uupsSrc },
  },
  settings: {
    optimizer: { enabled: true, runs: 200 },
    evmVersion: "paris",
    viaIR: true,
    outputSelection: { "*": { "*": ["abi", "evm.bytecode.object"] } },
  },
};

const out = JSON.parse(solc.compile(JSON.stringify(input)));
if (out.errors) {
  let fatal = false;
  for (const e of out.errors) {
    if (e.severity === "error") {
      console.error(e.formattedMessage || e.message);
      fatal = true;
    }
  }
  if (fatal) process.exit(1);
}

const c = out.contracts["PnsRegistryUpgradeable.sol"].PnsRegistryUpgradeable;
const hex = c.evm.bytecode.object;
if (!hex || hex.length < 200) {
  console.error("empty bytecode");
  process.exit(1);
}
const destDir = path.join(dir, "..", "crates", "vapurr-zmail", "src");
fs.writeFileSync(path.join(destDir, "pns_upgradeable_impl.hex"), hex + "\n");
fs.writeFileSync(path.join(destDir, "pns_upgradeable_impl.abi.json"), JSON.stringify(c.abi, null, 2) + "\n");
console.log("wrote pns_upgradeable_impl bytes", hex.length / 2);

if (out.errors) {
  for (const e of out.errors) {
    if (e.severity === "warning") console.warn(e.formattedMessage || e.message);
  }
}

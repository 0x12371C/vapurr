import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const lpSrc = fs.readFileSync(path.join(dir, "HouseLpUpgradeable.sol"), "utf8");
const swapSrc = fs.readFileSync(path.join(dir, "HouseSwapUpgradeable.sol"), "utf8");
const initSrc = fs.readFileSync(path.join(dir, "proxy", "Initializable.sol"), "utf8");
const uupsSrc = fs.readFileSync(path.join(dir, "proxy", "UUPSUpgradeable.sol"), "utf8");

const input = {
  language: "Solidity",
  sources: {
    "HouseLpUpgradeable.sol": { content: lpSrc },
    "HouseSwapUpgradeable.sol": { content: swapSrc },
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

function writeOut(fileKey, contractName, destBase) {
  const c = out.contracts[fileKey][contractName];
  const hex = c.evm.bytecode.object;
  if (!hex || hex.length < 200) {
    console.error("empty bytecode for", contractName);
    process.exit(1);
  }
  const destDir = path.join(dir, "..", "crates", "vapurr-econ", "src");
  fs.writeFileSync(path.join(destDir, destBase + ".hex"), hex + "\n");
  fs.writeFileSync(path.join(destDir, destBase + ".abi.json"), JSON.stringify(c.abi, null, 2) + "\n");
  console.log("wrote", destBase, "bytes", hex.length / 2);
}

writeOut("HouseLpUpgradeable.sol", "HouseLpUpgradeable", "house_lp_upgradeable_impl");
writeOut("HouseSwapUpgradeable.sol", "HouseSwapUpgradeable", "house_swap_upgradeable_impl");

if (out.errors) {
  for (const e of out.errors) {
    if (e.severity === "warning") console.warn(e.formattedMessage || e.message);
  }
}

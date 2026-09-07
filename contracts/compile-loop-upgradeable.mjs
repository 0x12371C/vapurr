import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const implSrc = fs.readFileSync(path.join(dir, "PusdLoopUpgradeable.sol"), "utf8");
const remitSrc = fs.readFileSync(path.join(dir, "Remittance.sol"), "utf8");
const initSrc = fs.readFileSync(path.join(dir, "proxy", "Initializable.sol"), "utf8");
const uupsSrc = fs.readFileSync(path.join(dir, "proxy", "UUPSUpgradeable.sol"), "utf8");
const proxySrc = fs.readFileSync(path.join(dir, "proxy", "ERC1967Proxy.sol"), "utf8");

const input = {
  language: "Solidity",
  sources: {
    "PusdLoopUpgradeable.sol": { content: implSrc },
    "Remittance.sol": { content: remitSrc },
    "proxy/Initializable.sol": { content: initSrc },
    "proxy/UUPSUpgradeable.sol": { content: uupsSrc },
    "proxy/ERC1967Proxy.sol": { content: proxySrc },
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

writeOut("PusdLoopUpgradeable.sol", "PusdLoopUpgradeable", "loop_upgradeable_impl");
writeOut("proxy/ERC1967Proxy.sol", "ERC1967Proxy", "erc1967_proxy");

if (out.errors) {
  for (const e of out.errors) {
    if (e.severity === "warning") console.warn(e.formattedMessage || e.message);
  }
}

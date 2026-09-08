import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const sources = {
  "GvFed.sol": { content: fs.readFileSync(path.join(dir, "GvFed.sol"), "utf8") },
  "IVapurrMinter.sol": { content: fs.readFileSync(path.join(dir, "IVapurrMinter.sol"), "utf8") },
};
const input = {
  language: "Solidity",
  sources,
  settings: {
    optimizer: { enabled: true, runs: 200 },
    evmVersion: "paris",
    outputSelection: { "*": { "*": ["abi"] } },
  },
};
function findImports(importPath) {
  const p = path.join(dir, importPath.replace(/^\.\//, ""));
  if (fs.existsSync(p)) return { contents: fs.readFileSync(p, "utf8") };
  return { error: "File not found: " + importPath };
}
const out = JSON.parse(solc.compile(JSON.stringify(input), { import: findImports }));
if (out.errors) {
  for (const e of out.errors) {
    if (e.severity === "error") {
      console.error(e.formattedMessage || e.message);
      process.exit(1);
    }
  }
}
const c = out.contracts["GvFed.sol"].BrowserStream;
if (!c || !c.abi) {
  console.error("missing BrowserStream abi; contracts=", Object.keys(out.contracts["GvFed.sol"] || {}));
  process.exit(1);
}
const dest = path.join(dir, "..", "crates", "vapurr-econ", "src", "browser_stream.abi.json");
fs.writeFileSync(dest, JSON.stringify(c.abi, null, 2) + "\n");
console.log("wrote", dest, "funcs", c.abi.filter((x) => x.type === "function").length);

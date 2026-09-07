import solc from "solc";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const dir = path.dirname(fileURLToPath(import.meta.url));
const src = fs.readFileSync(path.join(dir, "DevFundStream.sol"), "utf8");
const input = {
  language: "Solidity",
  sources: { "DevFundStream.sol": { content: src } },
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
const c = out.contracts["DevFundStream.sol"].DevFundStream;
if (!c || !c.abi) {
  console.error("missing DevFundStream abi");
  process.exit(1);
}
const dest = path.join(dir, "..", "crates", "vapurr-econ", "src", "dev_fund_stream.abi.json");
fs.writeFileSync(dest, JSON.stringify(c.abi, null, 2) + "\n");
console.log("wrote", dest, "funcs", c.abi.filter((x) => x.type === "function").length);

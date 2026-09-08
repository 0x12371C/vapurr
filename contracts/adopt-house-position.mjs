import { ethers } from "ethers";
import fs from "fs";

const RPC = "https://rpc.testnet.chain.robinhood.com";
const provider = new ethers.JsonRpcProvider(RPC, { chainId: 46630, name: "rhc-testnet" });
const key = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/.oliver-deployer.json", "utf8"));
const wallet = new ethers.Wallet(key.priv, provider);

const HOUSE_LP_PROXY = "0x603AaDFCD483aC196E2bcB158989dD5d38B24336";
const abi = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/house_lp_upgradeable_impl.abi.json", "utf8"));

// From live (old bare-impl) HouseLp.snapshot() (2026-09-07) — recording only, mints nothing.
const ADOPT = {
  tokenId: 2273n,
  tickLower: -1860,
  tickUpper: 1860,
  liquidity: 2207150961477685069479936n,
  poolId: "0x4480433a3e442ad8d328422d099be4dc2217492c727e13a188a0d0a7b807d97b",
};

async function main() {
  const houseLp = new ethers.Contract(HOUSE_LP_PROXY, abi, wallet);
  const tx = await houseLp.adopt(ADOPT.tokenId, ADOPT.tickLower, ADOPT.tickUpper, ADOPT.liquidity, ADOPT.poolId);
  const receipt = await tx.wait();
  console.log("adopt() tx", receipt.hash, "status", receipt.status);

  const snap = await houseLp.snapshot();
  console.log("tokenId now", snap.tokenId_.toString());
  console.log("liquidity now", snap.liquidity_.toString());
}

main().catch(e => { console.error(e); process.exit(1); });

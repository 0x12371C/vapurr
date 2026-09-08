import { ethers } from "ethers";
import fs from "fs";

const RPC = "https://rpc.testnet.chain.robinhood.com";
const provider = new ethers.JsonRpcProvider(RPC, { chainId: 46630, name: "rhc-testnet" });
const key = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/.oliver-deployer.json", "utf8"));
const wallet = new ethers.Wallet(key.priv, provider);

const PROXY = "0x07d1085b545d5e1f55668a6a2EA9332233AaeC69";
const OPERATOR = "0xe718e24b8d438a26cf39226854ed0b22db0ca56f";

const abi = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/loop_upgradeable_impl.abi.json", "utf8"));
const hex = "0x" + fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/loop_upgradeable_impl.hex", "utf8").trim();

async function main() {
  console.log("deployer", wallet.address);

  // 1. deploy v2 implementation
  const factory = new ethers.ContractFactory(abi, hex, wallet);
  const impl = await factory.deploy();
  await impl.waitForDeployment();
  const implAddr = await impl.getAddress();
  console.log("V2 IMPL", implAddr, impl.deploymentTransaction().hash);

  // 2. point the existing proxy at it (UUPS, authorized by owner)
  const proxied = new ethers.Contract(PROXY, abi, wallet);
  const tx = await proxied.upgradeToAndCall(implAddr, "0x");
  const rc = await tx.wait();
  console.log("upgradeToAndCall tx", rc.hash, "status", rc.status);

  // 3. prove it took and that the previously-reverting read now works
  const ro = new ethers.Contract(PROXY, abi, provider);
  console.log("oliverVersion()", (await ro.oliverVersion()).toString());
  const s = await ro.snapshot(OPERATOR);
  console.log("snapshot px      ", s.px.toString());
  console.log("snapshot cash    ", s.cash.toString());
  console.log("snapshot ltvBps  ", s.ltvBps.toString());
  console.log("snapshot bootBps ", s.bootBps.toString());
  console.log("snapshot cashTgt ", s.cashTarget.toString());
  console.log("market/vapurr/pusd", s.market_, s.vapurrToken, s.pusdToken);

  // 4. confirm storage survived the upgrade
  console.log("owner() after upgrade", await ro.owner());
}

main().catch(e => { console.error(e); process.exit(1); });

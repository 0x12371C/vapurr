import { ethers } from "ethers";
import fs from "fs";

const RPC = "https://rpc.testnet.chain.robinhood.com";
const provider = new ethers.JsonRpcProvider(RPC, { chainId: 46630, name: "rhc-testnet" });
const key = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/.oliver-deployer.json", "utf8"));
const wallet = new ethers.Wallet(key.priv, provider);

const MARKET = "0x47Aca5292423e2133A3eE983aB38291de3983617";

async function main() {
  console.log("deployer", wallet.address);
  const bal = await provider.getBalance(wallet.address);
  console.log("balance", ethers.formatEther(bal));

  const implHex = "0x" + fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/loop_upgradeable_impl.hex", "utf8").trim();
  const proxyHex = "0x" + fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/erc1967_proxy.hex", "utf8").trim();
  const implAbi = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/loop_upgradeable_impl.abi.json", "utf8"));

  // 1. Deploy implementation
  const implFactory = new ethers.ContractFactory(implAbi, implHex, wallet);
  const impl = await implFactory.deploy();
  await impl.waitForDeployment();
  const implAddr = await impl.getAddress();
  const implTx = impl.deploymentTransaction();
  console.log("IMPL_ADDR", implAddr);
  console.log("IMPL_TX", implTx.hash);

  // 2. Encode initialize(market, owner) calldata
  const iface = new ethers.Interface(implAbi);
  const initData = iface.encodeFunctionData("initialize", [MARKET, wallet.address]);

  // 3. Deploy ERC1967Proxy(impl, initData)
  const proxyAbi = [
    { "inputs": [ {"internalType":"address","name":"implementation_","type":"address"}, {"internalType":"bytes","name":"initData","type":"bytes"} ], "stateMutability":"payable", "type":"constructor" }
  ];
  const proxyFactory = new ethers.ContractFactory(proxyAbi, proxyHex, wallet);
  const proxy = await proxyFactory.deploy(implAddr, initData);
  await proxy.waitForDeployment();
  const proxyAddr = await proxy.getAddress();
  const proxyTx = proxy.deploymentTransaction();
  console.log("PROXY_ADDR", proxyAddr);
  console.log("PROXY_TX", proxyTx.hash);

  const balAfter = await provider.getBalance(wallet.address);
  console.log("balance after", ethers.formatEther(balAfter));

  fs.writeFileSync("C:/Users/jfren/vapurr/.oliver-deploy-result.json", JSON.stringify({
    market: MARKET, implAddr, implTx: implTx.hash, proxyAddr, proxyTx: proxyTx.hash, owner: wallet.address
  }, null, 2));
}

main().catch(e => { console.error(e); process.exit(1); });

import { ethers } from "ethers";
import fs from "fs";

const RPC = "https://rpc.testnet.chain.robinhood.com";
const provider = new ethers.JsonRpcProvider(RPC, { chainId: 46630, name: "rhc-testnet" });
const key = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/.oliver-deployer.json", "utf8"));
const wallet = new ethers.Wallet(key.priv, provider);

const PUSD = "0xBe71EF3e1b49ec35b4C3A80c257342A39CEEE42e";

const proxyAbi = [
  { "inputs": [ {"internalType":"address","name":"implementation_","type":"address"}, {"internalType":"bytes","name":"initData","type":"bytes"} ], "stateMutability":"payable", "type":"constructor" }
];
const proxyHex = "0x" + fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/erc1967_proxy.hex", "utf8").trim();

async function deployImplAndProxy(label, abiPath, hexPath, initArgs) {
  const abi = JSON.parse(fs.readFileSync(abiPath, "utf8"));
  const hex = "0x" + fs.readFileSync(hexPath, "utf8").trim();
  const implFactory = new ethers.ContractFactory(abi, hex, wallet);
  const impl = await implFactory.deploy();
  await impl.waitForDeployment();
  const implAddr = await impl.getAddress();
  console.log(label, "IMPL", implAddr, impl.deploymentTransaction().hash);

  const iface = new ethers.Interface(abi);
  const initData = iface.encodeFunctionData("initialize", initArgs);
  const proxyFactory = new ethers.ContractFactory(proxyAbi, proxyHex, wallet);
  const proxy = await proxyFactory.deploy(implAddr, initData);
  await proxy.waitForDeployment();
  const proxyAddr = await proxy.getAddress();
  console.log(label, "PROXY", proxyAddr, proxy.deploymentTransaction().hash);

  // read back through the proxy
  const c = new ethers.Contract(proxyAddr, abi, provider);
  const stats = await c.stats();
  console.log(label, "stats() n/pot/top/min/minOut:", stats.map(x => x.toString()).join(" "));
  console.log(label, "pusd()", await c.pusd(), "owner()", await c.owner());

  return { implAddr, proxyAddr };
}

async function main() {
  console.log("deployer", wallet.address, "balance", ethers.formatEther(await provider.getBalance(wallet.address)));

  const outbid = await deployImplAndProxy(
    "Outbid",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/outbid_upgradeable_impl.abi.json",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/outbid_upgradeable_impl.hex",
    [PUSD, wallet.address]
  );

  const ketlist = await deployImplAndProxy(
    "KetList",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/ketlist_upgradeable_impl.abi.json",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/ketlist_upgradeable_impl.hex",
    [PUSD, wallet.address]
  );

  const result = { outbid, ketlist };
  fs.writeFileSync("C:/Users/jfren/vapurr/.boards-deploy-result.json", JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
  console.log("balance after", ethers.formatEther(await provider.getBalance(wallet.address)));
}

main().catch(e => { console.error(e); process.exit(1); });

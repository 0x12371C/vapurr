import { ethers } from "ethers";
import fs from "fs";

const RPC = "https://rpc.testnet.chain.robinhood.com";
const provider = new ethers.JsonRpcProvider(RPC, { chainId: 46630, name: "rhc-testnet" });
const key = JSON.parse(fs.readFileSync("C:/Users/jfren/vapurr/.oliver-deployer.json", "utf8"));
const wallet = new ethers.Wallet(key.priv, provider);

const MARKET = "0x47Aca5292423e2133A3eE983aB38291de3983617";
const POSM = "0x58daec3116aae6D93017bAAea7749052E8a04fA7";
const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3";
const PM = "0x8366a39cc670b4001a1121b8f6a443a643e40951";
const FEE = 3000;
const TICK_SPACING = 60;
const HOUSE_OWNER = "0xe718e24b8d438a26cf39226854ed0b22db0ca56f";

// From live HouseLp.snapshot() (2026-09-07)
const ADOPT = {
  tokenId: 2273n,
  tickLower: -1860,
  tickUpper: 1860,
  liquidity: 2207150961477685069479936n,
  poolId: "0x4480433a3e442ad8d328422d099be4dc2217492c727e13a188a0d0a7b807d97b",
};

const proxyAbi = [
  { "inputs": [ {"internalType":"address","name":"implementation_","type":"address"}, {"internalType":"bytes","name":"initData","type":"bytes"} ], "stateMutability":"payable", "type":"constructor" }
];
const proxyHex = "0x" + fs.readFileSync("C:/Users/jfren/vapurr/crates/vapurr-econ/src/erc1967_proxy.hex", "utf8").trim();

async function deployImplAndProxy(label, abiPath, hexPath, initFn, initArgs) {
  const abi = JSON.parse(fs.readFileSync(abiPath, "utf8"));
  const hex = "0x" + fs.readFileSync(hexPath, "utf8").trim();
  const implFactory = new ethers.ContractFactory(abi, hex, wallet);
  const impl = await implFactory.deploy();
  await impl.waitForDeployment();
  const implAddr = await impl.getAddress();
  console.log(label, "IMPL", implAddr, impl.deploymentTransaction().hash);

  const iface = new ethers.Interface(abi);
  const initData = iface.encodeFunctionData(initFn, initArgs);
  const proxyFactory = new ethers.ContractFactory(proxyAbi, proxyHex, wallet);
  const proxy = await proxyFactory.deploy(implAddr, initData);
  await proxy.waitForDeployment();
  const proxyAddr = await proxy.getAddress();
  console.log(label, "PROXY", proxyAddr, proxy.deploymentTransaction().hash);

  return { implAddr, proxyAddr, abi };
}

async function main() {
  console.log("deployer", wallet.address, "balance", ethers.formatEther(await provider.getBalance(wallet.address)));

  const house = await deployImplAndProxy(
    "HouseLp",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/house_lp_upgradeable_impl.abi.json",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/house_lp_upgradeable_impl.hex",
    "initialize",
    [MARKET, POSM, PERMIT2, FEE, TICK_SPACING, wallet.address]
  );

  const swap = await deployImplAndProxy(
    "HouseSwap",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/house_swap_upgradeable_impl.abi.json",
    "C:/Users/jfren/vapurr/crates/vapurr-econ/src/house_swap_upgradeable_impl.hex",
    "initialize",
    [MARKET, PM, FEE, TICK_SPACING, wallet.address]
  );

  const pns = await deployImplAndProxy(
    "Pns",
    "C:/Users/jfren/vapurr/crates/vapurr-zmail/src/pns_upgradeable_impl.abi.json",
    "C:/Users/jfren/vapurr/crates/vapurr-zmail/src/pns_upgradeable_impl.hex",
    "initialize",
    [wallet.address]
  );

  const result = {
    houseLp: { impl: house.implAddr, proxy: house.proxyAddr },
    houseSwap: { impl: swap.implAddr, proxy: swap.proxyAddr },
    pns: { impl: pns.implAddr, proxy: pns.proxyAddr },
  };
  fs.writeFileSync("C:/Users/jfren/vapurr/.house-pns-deploy-result.json", JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));

  console.log("balance after", ethers.formatEther(await provider.getBalance(wallet.address)));
}

main().catch(e => { console.error(e); process.exit(1); });

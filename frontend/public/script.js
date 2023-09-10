// load items from global imported CosmWasmJS
const { CosmWasmClient, setupWebKeplr } = CosmWasmJS

const RPC_URL = "https://rpc.atlantic-2.seinetwork.io"
const CONTRACT_ADDRESS = "sei1hc9xmnc6lmckxu29tk35arc02cfxwk24nphjruh2x9m8f2ap3kese7jeh8"
const ATHLETE_ADDRESS = "sei1njm50dvj0w06du79mkuq2r3wpwmsh05tndenjc"
const CHAIN_ID = "atlantic-2";

const COSMWASM_CONFIG = {
	chainId: CHAIN_ID,
  rpcEndpoint: RPC_URL,
  prefix: "sei",
}

var MAIN_ACCOUNT = " "
var KEPLR_CLIENT = null
var READ_CLIENT = null

window.onload = async () => {
	if (!window.keplr) {
		alert("Please install keplr extension to use this app");
	} else {
		// CosmWasm load
		await window.keplr.enable(CHAIN_ID);
		const offlineSigner = window.keplr.getOfflineSigner(CHAIN_ID);
		const accounts = await offlineSigner.getAccounts();
		MAIN_ACCOUNT = accounts[0].address;
		READ_CLIENT = await CosmWasmClient.connect(RPC_URL);
		KEPLR_CLIENT = await setupWebKeplr(COSMWASM_CONFIG);

		await loadAthleteShareData()
	}
}

async function buyShare() {
	const sharePrice = await queryContract({ GetPriceOnCurve: { owner: ATHLETE_ADDRESS } })
	const txInfo = await broadcastExecuteTx({ BuyPriceOnCurve: { owner: ATHLETE_ADDRESS } }, sharePrice.message)
	console.log("Buy share tx info", txInfo)
	alert(`Successfully purchased share! Tx Hash: ${txInfo.transactionHash}`)
	await loadAthleteShareData()
}

async function sellShare() {
	const txInfo = await broadcastExecuteTx({ SellPriceOnCurve: { owner: ATHLETE_ADDRESS } })
	console.log("Sell share tx info", txInfo)
	alert(`Successfully sold share! Tx Hash: ${txInfo.transactionHash}`)
	await loadAthleteShareData()
}

async function claimPerk() {
	const txInfo = await broadcastExecuteTx({ ClaimPerk: { owner: ATHLETE_ADDRESS, perk_id: 0 } })
	console.log("Claim perk tx info", txInfo)
	alert(`Successfully claimed perk! Tx Hash: ${txInfo.transactionHash}`)
	document.getElementById("perk-claim-alert").textContent = "You have 0 perks to claim"
	document.getElementById("perk-claim-button").disabled = true
}

async function loadAthleteShareData() {
	// Athlete stats init
	const sharePrice = await queryContract({ GetPriceOnCurve: { owner: ATHLETE_ADDRESS } })
	console.log("Share price response", sharePrice.message)
	document.getElementById("athlete-stats-share-price").textContent = `${sharePrice.message} usei`

	const sharesOwned = await queryContract({ GetBalanceOfHolder: { owner: ATHLETE_ADDRESS, holder: MAIN_ACCOUNT } })
	console.log("Holder share balance response", sharesOwned.message)
	document.getElementsByClassName("athlete-stats-shares-owned")[0].textContent = sharesOwned.message
	document.getElementsByClassName("athlete-stats-shares-owned")[1].textContent = sharesOwned.message
}

async function broadcastExecuteTx(jsonMsg, useiAmount) {
	if (!KEPLR_CLIENT) throw "Kepler client not set up"

	const txInfo = getExecuteTxMsgAndFee(jsonMsg, useiAmount)
	const result = await KEPLR_CLIENT.signAndBroadcast(MAIN_ACCOUNT, [txInfo[0]], txInfo[1], "")
	return result
}

async function signExecuteTx(jsonMsg) {
	if (!KEPLR_CLIENT) throw "Kepler client not set up"

	const txInfo = getExecuteTxMsgAndFee(jsonMsg)
	const result = await KEPLR_CLIENT.sign(MAIN_ACCOUNT, [txInfo[0]], txInfo[1], "")
	return result
}

async function queryContract(jsonMsg) {
	if (!READ_CLIENT) throw "Read client not set up"

	const resp = await READ_CLIENT.queryContractSmart(CONTRACT_ADDRESS, jsonMsg)
	return resp
}

function getExecuteTxMsgAndFee(jsonMsg, useiAmount = 0) {
	if (typeof jsonMsg != "object") throw "jsonMsg is not object";
	const fee = {
		amount: [{ amount: '0.1', denom: 'usei' }],
		gas: "200000"
	}

	const funds = [{
		amount: String(useiAmount),
		denom: "usei"
	}]

	const msg = {
		typeUrl: "/cosmwasm.wasm.v1.MsgExecuteContract",
		value: {
			sender: MAIN_ACCOUNT,
			contract: CONTRACT_ADDRESS,
			msg: btoa(JSON.stringify(jsonMsg)),
		}
	}

	if (useiAmount > 0) {
		msg.value.funds = funds
	}

	return [msg, fee]
}

function progressMainContent() {
	document.getElementById("main-content").setAttribute("useMap", "#img-map-2")
	document.getElementById("main-content").src = "https://cdn.glitch.global/b9507eea-b8af-4568-b3b2-8ef80ae28aa8/athlete_search.jpeg?v=1694365586615"
}

function setAthleteModalVisibility(visible) {
	document.getElementById("athlete-modal").style.display = visible ? "block" : "none"
	document.getElementById("athlete-modal-bg").style.display = visible ? "block" : "none"
}

// Testing functions

function testExecuteMsg() {
	const [msg, fee] = getExecuteTxMsgAndFee({ BuyPriceOnCurve: { owner: MAIN_ACCOUNT } })
	KEPLR_CLIENT.sign(MAIN_ACCOUNT, [msg], fee, "")
}

// ~~~ IMPORTS ~~~ //
use cosmwasm_std::{
	entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Storage, Addr, Empty, StdError, BankMsg,
	coins, Event
};

use serde::{ Deserialize, Serialize };

use cw_storage_plus::{ Map, Item };


// ~~~ STRUCTS ~~~ //

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum QueryMsg {
	GetPriceOnCurve { owner: Addr },
	GetSupplyOfOwner { owner: Addr },
	GetBalanceOfHolder { holder: Addr, owner: Addr }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum ExecuteMsg {
	AllowAthleteJoin { athlete: Addr },
	RegisterSelf { perk_settings: Vec<(u16, u64)>, first_name: String, last_name: String },
	BuyPriceOnCurve { owner: Addr },
    SellPriceOnCurve { owner: Addr },
	ClaimPerk { owner: Addr, perk_id: u64 },
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NumResp {
	message: u128,
}

// ~~~ STATE ~~~ //

pub const SUPPLY:Map<Addr, u128> = Map::new("supply"); // supply of each athlete's pass
pub const BALANCE:Map<(Addr, Addr), Vec<u64>> = Map::new("balance"); // (holder, owner) => vec of timestamps purchased
pub const ALLOWED_ADDRS:Map<Addr, bool> = Map::new("allowed addrs"); // the addresses that can register
pub const AVAILABLE_PERKS:Map<Addr, Vec<(u16, u64)>> = Map::new("available perks"); // owner => (shares to hold, how long to hold shares)
pub const OWNER:Item<Addr> = Item::new("owner"); // deployer of contract

// ~~~ MAIN FUNCS ~~~ //

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
	OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new())
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
		GetPriceOnCurve { owner } => to_binary(&query::get_price_on_curve(deps.storage, owner, false)?),
		GetBalanceOfHolder { holder, owner } => to_binary(&query::get_balance_of_holder(deps.storage, holder, owner)?),
		GetSupplyOfOwner { owner } => to_binary(&query::get_supply_of_owner(deps.storage, owner)?),
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> StdResult<Response> {
	use ExecuteMsg::*;

    match msg {
		AllowAthleteJoin { athlete } => exec::allow_athlete_join(deps.storage, info.sender, athlete),
		ClaimPerk { owner, perk_id } => exec::claim_perk(deps.storage, env.block.time.seconds(), perk_id, owner, info.sender),
		RegisterSelf { first_name, last_name, perk_settings } => exec::register_addr(deps.storage, info.sender, first_name, last_name, perk_settings),
		BuyPriceOnCurve { owner } => exec::buy_price_on_curve(deps.storage, env.block.time.seconds(), owner, info.sender, u128::from(info.funds[0].amount)),
		SellPriceOnCurve { owner } => exec::sell_price_on_curve(deps.storage, owner, info.sender),
    }
}

// ~~~ SECOND. FUNCS ~~~ //

mod query {
    use super::*;

	pub fn get_price_on_curve(stor: &dyn Storage, owner: Addr, sell_mode: bool) -> StdResult<NumResp> {
		let mut current_quant: u128 = SUPPLY.load(stor, owner)?;
		let mut price;
		// todo: add the surge pricing
		if sell_mode == true {
			price = current_quant * current_quant;
		} else {
			current_quant += 1;
			price = current_quant * current_quant;
		}
		price *= 100;
		// load price into response
		let resp = NumResp {
			message: price
		};

		Ok(resp)
	}

	pub fn get_balance_of_holder(stor: &dyn Storage, holder: Addr, owner: Addr) -> StdResult<NumResp> {
		let passes = BALANCE.may_load(stor, (holder, owner)).unwrap().unwrap_or_default();
		let resp = NumResp {
			message: passes.len() as u128
		};

		Ok(resp)
	}

	pub fn get_supply_of_owner(stor: &dyn Storage, owner: Addr) -> StdResult<NumResp> {
		let sup = SUPPLY.load(stor, owner)?;
		let resp = NumResp {
			message: sup
		};

		Ok(resp)
	}
}

// TODO: Add multi sell and multi buy
mod exec {
	use super::*;

	pub fn allow_athlete_join(stor: &mut dyn Storage, msg_sender: Addr, athlete: Addr) -> StdResult<Response> {
		let contract_owner = OWNER.load(stor)?;
		if msg_sender != contract_owner {
			return Err(StdError::generic_err("Only contract owner can execute"));
		}

		ALLOWED_ADDRS.save(stor, athlete, &true)?;

		Ok(Response::new())
	}

	pub fn register_addr(stor: &mut dyn Storage, owner: Addr, first_name: String, last_name: String, perk_settings: Vec<(u16, u64)>) -> StdResult<Response> {
		let is_allowed = ALLOWED_ADDRS.may_load(stor, owner.clone())?;
		if is_allowed == None {
			return Err(StdError::generic_err("Sender is not whitelisted"));
		}

		if perk_settings.len() == 0 {
			return Err(StdError::generic_err("Must include at least one perk"));
		}

		// remove to keep storage small
		ALLOWED_ADDRS.remove(stor, owner.clone());

		// set default storage values
		SUPPLY.save(stor, owner.clone(), &(0 as u128))?;
		AVAILABLE_PERKS.save(stor, owner, &perk_settings)?;

		let event = Event::new("athlete_joined")
			.add_attribute("last_name", first_name)
			.add_attribute("first_name", last_name);

		Ok(Response::new().add_event(event))
	}

	pub fn buy_price_on_curve(stor: &mut dyn Storage, timestamp: u64, owner: Addr, buyer: Addr, msg_value: u128) -> StdResult<Response> {
		let price: NumResp = query::get_price_on_curve(stor, owner.clone(), false)?;

		if msg_value < price.message {
			return Err(StdError::generic_err("Not enough funds transferred"));
		}

		// proccess fees - 5% to pass owner and 5% to platform
		let owner_fee = (price.message * 5) / 100;
		let owner_fee_msg = BankMsg::Send {
			to_address: owner.to_string(),
			amount: coins(owner_fee, "usei"),
		};

		// process fee to platform owner
		let owner_addr = OWNER.load(stor)?;
		let platform_fee_msg = BankMsg::Send {
			to_address: owner_addr.to_string(),
			amount: coins(owner_fee, "usei")
		};

		BALANCE.update(stor, (buyer.clone(), owner.clone()), |maybe_passes| -> StdResult<_> {
			let mut passes = maybe_passes.unwrap_or_default();
			if passes.len() == 0 {
				passes = Vec::new();
			}
			passes.push(timestamp);
			Ok(passes)
		})?;

		SUPPLY.update(stor, owner.clone(), |val| -> StdResult<_> {
			Ok(val.unwrap_or_default() + 1)
		})?;

		// If applicable, send fees
		if owner_fee > 0 {
			return Ok(Response::new().add_message(platform_fee_msg).add_message(owner_fee_msg));
		}

		Ok(Response::new())
	}

	pub fn sell_price_on_curve(stor: &mut dyn Storage, owner: Addr, seller: Addr) -> StdResult<Response> {
		let price: NumResp = query::get_price_on_curve(stor, owner.clone(), true)?;

		// there is a total 10% fee
		let transfer_amt = price.message - ((price.message * 10) / 100);
		let transfer_msg = BankMsg::Send {
			to_address: seller.to_string(),
			amount: coins(transfer_amt, "usei")
		};

		BALANCE.update(stor, (seller.clone(), owner.clone()), |maybe_passes| -> StdResult<_> {
			let mut passes = maybe_passes.unwrap_or_default();
			if passes.len() == 0 {
				return Err(StdError::generic_err("No pass of owner found for seller"));
			}
			passes.pop();
			Ok(passes)
		})?;


		SUPPLY.update(stor, owner.clone(), |val| -> StdResult<_> {
			Ok(val.unwrap_or_default() - 1)
		})?;

		Ok(Response::new().add_message(transfer_msg))
	}

	pub fn claim_perk(stor: &mut dyn Storage, blck_timestamp: u64, perk_id: u64, owner: Addr, claimer: Addr) -> StdResult<Response> {
		let bal = query::get_balance_of_holder(stor, claimer.clone(), owner.clone())?;
		if bal.message == 0 {
			return Err(StdError::generic_err("Caller does not have shares of owner"));
		}
		let perk_settings = AVAILABLE_PERKS.load(stor, owner.clone())?;
		let owned_shares = BALANCE.load(stor, (claimer.clone(), owner.clone()))?;
		let mut qualifying_shares = 0;

		for share_acq_time in owned_shares {
			if share_acq_time <= (blck_timestamp - perk_settings[perk_id as usize].1) {
				qualifying_shares += 1;
			}
		}

		if qualifying_shares < perk_settings[perk_id as usize].0 {
			return Err(StdError::generic_err("Not enough shares and/or not held for long enough"));
		}

		let event = Event::new("perk_claimed")
			.add_attribute("owner", owner.clone())
			.add_attribute("claimer", claimer.clone())
			.add_attribute("perk_id", perk_id.to_string());

		Ok(Response::new().add_event(event))
	}
}

// ~~~ TESTS ~~~ //

#[cfg(test)]
mod tests {
	use cw_multi_test::{ App, ContractWrapper, Executor };
    use cosmwasm_std::{ Coin, BlockInfo };

    use super::*;

	// Helper Functions for Test

	fn buy_price_on_curve_execute(app: &mut App, contract: Addr, owner: Addr, buyer: Addr, coins_amt: Vec<Coin>) {
		let _ = app
			.execute_contract(
				buyer,
				contract,
				&ExecuteMsg::BuyPriceOnCurve { owner: owner },
				&coins_amt
			)
			.unwrap();
	}

	fn sell_price_on_curve_execute(app: &mut App, contract: Addr, owner: Addr, seller: Addr) {
		let _ = app
			.execute_contract(
				seller,
				contract,
				&ExecuteMsg::SellPriceOnCurve { owner: owner },
				&[]
			)
			.unwrap();
	}

	fn get_supply_of_owner_query(app: &App, contract: Addr, owner: Addr) -> u128 {
		let resp: NumResp = app.wrap()
            .query_wasm_smart(contract, &QueryMsg::GetSupplyOfOwner { owner: owner }).unwrap();

		return resp.message;
	}

	fn get_price_on_curve_query(app: &App, contract: Addr, owner: Addr) -> u128 {
		let resp: NumResp = app.wrap()
            .query_wasm_smart(contract, &QueryMsg::GetPriceOnCurve { owner: owner }).unwrap();
		return resp.message
	}

	fn get_balance_of_holder_query(app: &App, contract: Addr, holder: Addr, owner: Addr) -> u128 {
		let resp: NumResp = app.wrap()
            .query_wasm_smart(contract, &QueryMsg::GetBalanceOfHolder { owner: owner, holder: holder})
            .unwrap();

		return resp.message;
	}

	fn query_addr_balance(app: &App, who: Addr) -> u128 {
		app.wrap().query_balance(who.to_string(), "usei").unwrap().amount.u128()
	}

    #[test]
    fn main_test() {
		// 1 million usei = 1 sei
		const UNIT_COIN_AMT: u128 = 10_u128.pow(6);
		const UNIT_COIN_DENOM: &str = "usei";

		// Initialize app
        let mut app = App::new(|router, _, storage | {
			router.bank
				.init_balance(storage, &Addr::unchecked("user"), coins(UNIT_COIN_AMT, UNIT_COIN_DENOM))
				.unwrap();
		});

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &Empty {},
                &[],
                "AthletiX",
                None,
            )
            .unwrap();

		// Begin actual testing

		println!("Whitelist Athlete's Address");
		let _ = app
			.execute_contract(
				Addr::unchecked("owner"),
				addr.clone(),
				&ExecuteMsg::AllowAthleteJoin { athlete: Addr::unchecked("athlete") },
				&[]
			)
			.unwrap();

		println!("Athlete Register");
		// register the athlete
		let _ = app
			.execute_contract(
				Addr::unchecked("athlete"),
				addr.clone(),
				&ExecuteMsg::RegisterSelf { first_name: String::from("A"), last_name: String::from("B"), perk_settings: vec![(1, 2)] },
				&[]
			)
			.unwrap();
		// check init supply
		assert_eq!(
			get_supply_of_owner_query(&app, addr.clone(), Addr::unchecked("athlete")),
			0
		);

		println!("Get Price of Share");
		let init_price = get_price_on_curve_query(&app, addr.clone(), Addr::unchecked("athlete"));
		assert_eq!(init_price, 100);

		println!("Purchase Share");
		// purchase share
		buy_price_on_curve_execute(&mut app, addr.clone(), Addr::unchecked("athlete"), Addr::unchecked("user"), coins(init_price, UNIT_COIN_DENOM));
		// check for share transfer
		assert_eq!(
			get_balance_of_holder_query(&app, addr.clone(), Addr::unchecked("user"), Addr::unchecked("athlete")),
			1
		);
		// check for price increase
		assert_eq!(
			get_price_on_curve_query(&app, addr.clone(), Addr::unchecked("athlete")),
			400
		);
		// check for fee transfer
		assert_eq!(
			query_addr_balance(&app, Addr::unchecked("athlete")),
			(init_price * 5 / 100)
		);
		assert_eq!(
			query_addr_balance(&app, Addr::unchecked("owner")),
			(init_price * 5 / 100)
		);

		println!("Claim perk");
		// increase block time by 2 sec so perk can be claimed
		let mut block_info:BlockInfo = app.block_info();
		block_info.time = block_info.time.plus_seconds(2);
		app.set_block(block_info);
		let _ = app
			.execute_contract(
				Addr::unchecked("user"),
				addr.clone(),
				&ExecuteMsg::ClaimPerk { owner: Addr::unchecked("athlete"), perk_id: 0 },
				&[]
			)
			.unwrap();

		println!("Purchase Share");
		let prev_bal = query_addr_balance(&app, Addr::unchecked("user"));
		sell_price_on_curve_execute(&mut app, addr.clone(), Addr::unchecked("athlete"), Addr::unchecked("user"));
		// check for share removal
		assert_eq!(
			get_balance_of_holder_query(&app, addr.clone(), Addr::unchecked("user"), Addr::unchecked("athlete")),
			0
		);
		// check funds transfer
		assert_eq!(
			(prev_bal + init_price) - ((init_price * 10) / 100),
			query_addr_balance(&app, Addr::unchecked("user"))
		);
		// check new price of share
		assert_eq!(
			get_price_on_curve_query(&app, addr.clone(), Addr::unchecked("athlete")),
			init_price
		);
    }
}

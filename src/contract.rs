#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Addr, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, CosmosMsg, BankMsg, QueryRequest, BankQuery, WasmMsg,
    Coin, AllBalanceResponse
};
use cw2::set_contract_version;
use cw_storage_plus::{U128Key};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, BalanceResponse as Cw20BalanceResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, PROJECTSTATES, ProjectState, BackerState,
                    PROJECT_SEQ};

use crate::market::{ExecuteMsg as AnchorMarket, Cw20HookMsg};                    

// version info for migration info
const CONTRACT_NAME: &str = "WEFUND";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = msg
        .admin
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(info.sender.clone());

    let wefund = msg
        .wefund
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(info.sender.clone());

    let anchor_market = msg
        .anchor_market
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(Addr::unchecked(
            String::from("terra1sepfj7s0aeg5967uxnfk4thzlerrsktkpelm5s")));
            // terra15dwd5mj8v59wpj0wvt233mf5efdff808c5tkal
    let aust_token = msg
        .aust_token
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(Addr::unchecked(
            String::from("terra1hzh9vpxhsk8253se0vv5jj6etdvxu3nv8z07zu")));
            // terra1ajt556dpzvjwl0kl5tzku3fc3p3knkg9mkv8jl
    let config = Config {
        owner, wefund, anchor_market, aust_token,
    };

    CONFIG.save(deps.storage, &config)?;
    PROJECT_SEQ.save(deps.storage, &Uint128::new(0))?;

    Ok(Response::new()
        .add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetConfig{ admin, wefund, anchor_market, aust_token } 
            => try_setconfig(deps, info, admin, wefund, anchor_market, aust_token),
        ExecuteMsg::AddProject { 
            project_name,
            project_createddate,
            project_description,
            project_teamdescription,
            project_category,
            project_subcategory,
            project_chain,
            project_collected,
            project_deadline,
            project_website,
            project_icon,
            project_email,
            project_whitepaper,
            creator_wallet,
        } => 
            try_addproject(deps, info, 
                project_name,
                project_createddate,
                project_description,
                project_teamdescription,
                project_category,
                project_subcategory,
                project_chain,
                project_collected,
                project_deadline,
                project_website,
                project_icon,
                project_email,
                project_whitepaper,
                creator_wallet,
            ),

        ExecuteMsg::Back2Project { project_id, backer_wallet } => 
            try_back2project(deps, info, project_id, backer_wallet),

        ExecuteMsg::CompleteProject{ project_id } =>
            try_completeproject(deps, _env, info, project_id ),

        ExecuteMsg::FailProject{ project_id } =>
            try_failproject(deps, _env, info, project_id),
        
        ExecuteMsg::RemoveProject{ project_id } =>
            try_removeproject(deps, info, project_id),
        
        ExecuteMsg::TransferAllCoins{wallet} =>
            try_transferallcoins(deps, _env, info, wallet),
    }
}
pub fn try_transferallcoins(deps:DepsMut, _env:Env, info:MessageInfo, wallet:String)
    -> Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner
    {
        return Err(ContractError::Unauthorized{});
    }
    //--------get all native coins and ust - 4 ----------------------
    let balance: AllBalanceResponse = deps.querier.query(
        &QueryRequest::Bank(BankQuery::AllBalances {
            address: _env.contract.address.to_string(),
        }
    ))?;

    let mut nativecoins:Vec<Coin> = Vec::new();
    for mut x in balance.amount
    {
        if x.denom == "uusd" {
            if x.amount.u128() < 4000000 {
                return Err(ContractError::NeedCoin{});
            }
            x.amount = Uint128::new(x.amount.u128() - 4000000);
        }
        nativecoins.push(x);
    }

    let bank_native = BankMsg::Send { 
        to_address: wallet.clone(),
        amount: nativecoins,
    };

    //--------transfer all aust--------------------------------
    let config = CONFIG.load(deps.storage).unwrap();

    let aust_balance: Cw20BalanceResponse = deps.querier.query_wasm_smart(
        config.aust_token.clone(),
        &Cw20QueryMsg::Balance{
            address: _env.contract.address.to_string(),
        }
    )?;
    let bank_aust = WasmMsg::Execute {
        contract_addr: String::from(config.aust_token),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: wallet,
            amount: aust_balance.balance,
        }).unwrap(),
        funds: Vec::new()
    };

    Ok(Response::new()
    .add_messages(vec![
        CosmosMsg::Bank(bank_native),
        CosmosMsg::Wasm(bank_aust)])
    .add_attribute("action", "trasnfer all coins")
    )
}
pub fn try_removeproject(deps:DepsMut, info:MessageInfo, project_id:Uint128)
    -> Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner
    {
        return Err(ContractError::Unauthorized{});
    }

    return remove_project(deps, project_id);
}
pub fn remove_project(deps:DepsMut, _project_id:Uint128)
    ->Result<Response, ContractError>
{
    let res = PROJECTSTATES.may_load(deps.storage, _project_id.u128().into());
    if res == Ok(None) {
        return Err(ContractError::NotRegisteredProject {});
    }
    PROJECTSTATES.remove(deps.storage, U128Key::new(_project_id.u128()));
    Ok(Response::new())
}
pub fn try_setconfig(deps:DepsMut, info:MessageInfo,
    admin:Option<String>, 
    wefund:Option<String>, 
    anchor_market:Option<String>, 
    aust_token:Option<String>
) -> Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner
    {
        return Err(ContractError::Unauthorized{});
    }
    
    let mut config = CONFIG.load(deps.storage).unwrap();

    config.owner =  admin
    .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
    .unwrap_or(config.owner);

    config.wefund = wefund
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(config.wefund);

    config.anchor_market = anchor_market
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(config.anchor_market);

    config.aust_token = aust_token
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok()) 
        .unwrap_or(config.aust_token);

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "SetConfig"))                                
}
pub fn try_completeproject(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _project_id: Uint128
) -> Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner
    {
        return Err(ContractError::Unauthorized{});
    }
    //--------Get project info----------------------------
    let x:ProjectState = PROJECTSTATES.load(deps.storage, _project_id.u128().into())?;

    //--------Checking project status-------------------------
    if x.project_done != Uint128::zero()
    {
        return Err(ContractError::AlreadyDoneFail{});
    }

    //---------calc project collection---------------------------
    let mut collected = 0;
    for backer in x.backer_states{
        collected += backer.ust_amount.amount.u128();
    }

    //---------calc total real backed money on smart contract----------------
    //----------map to vec-----------------------
    let all: StdResult<Vec<_>> = PROJECTSTATES.range(deps.storage, None, None, 
        cosmwasm_std::Order::Ascending).collect();
    let all = all.unwrap();

    let mut total_real_backed = 0;
    for x in all{
        let prj = x.1;
        if prj.project_done == Uint128::zero() //exclude done or fail project
        {
            for y in prj.backer_states{
                total_real_backed += y.ust_amount.amount.u128();
            }
        }
    }
  
    //----------load config and read aust token address-----------------
    let config = CONFIG.load(deps.storage).unwrap();
    
    //--------get aust balance---------------------
    let aust_balance: Cw20BalanceResponse = deps.querier.query_wasm_smart(
        config.aust_token.clone(),
        &Cw20QueryMsg::Balance{
            address: _env.contract.address.to_string(),
        }
    )?;

    //----------calc declaim aust amount---aust*(collcted/total)-----------
    let withdraw_amount = 
        Uint128::from(aust_balance.balance.u128() 
        * collected / total_real_backed);


    //----ask aust_token for transfer to anchor martket and execute redeem_stable ----------
    let withdraw = WasmMsg::Execute {
        contract_addr: String::from(config.aust_token),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: config.anchor_market.to_string(),
            msg: to_binary(&Cw20HookMsg::RedeemStable{}).unwrap(), //redeem_stable{}
            amount: withdraw_amount
        }).unwrap(),
        funds: Vec::new()
    };

    //---------send to creator wallet-------------
    let ust_collected = Coin::new(collected, "uusd");
    let send2_creator = BankMsg::Send { 
        to_address: x.creator_wallet,
        amount: vec![ust_collected] 
    };

    // remove_project(deps, _project_id);
    //-----update project state to DONE----------------------------
    // let act = |a: Option<ProjectState>| -> StdResult<ProjectState> { 
    //     Ok(ProjectState {
    //         project_id: a.clone().unwrap().project_id,
    //         project_name: a.clone().unwrap().project_name,
    //         project_wallet: a.clone().unwrap().project_wallet,
    //         project_collected: a.clone().unwrap().project_collected,
    //         project_website: a.clone().unwrap().project_website,
    //         project_about: a.clone().unwrap().project_about,
    //         project_email: a.clone().unwrap().project_email,
    //         project_ecosystem: a.clone().unwrap().project_ecosystem,
    //         project_category: a.clone().unwrap().project_category,
    //         creator_wallet: a.clone().unwrap().creator_wallet,
    //         project_needback: a.clone().unwrap().project_needback,
    //         project_done: Uint128::new(1), //done
    //         backer_states: a.clone().unwrap().backer_states,
    //     })
    // };
    // PROJECTSTATES.update(deps.storage, _project_id.u128().into(), act)?;
    
    PROJECTSTATES.update(deps.storage, _project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_done = Uint128::new(1); //done
            Ok(project)
        }
    })?;

    Ok(Response::new()
    .add_messages(vec![
        CosmosMsg::Wasm(withdraw),
        CosmosMsg::Bank(send2_creator)])
    .add_attribute("action", "project failed")
    )
}
pub fn try_failproject(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _project_id: Uint128
) -> Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner
    {
        return Err(ContractError::Unauthorized{});
    }
    //--------Get project info----------------------------
    let x:ProjectState = PROJECTSTATES.load(deps.storage, _project_id.u128().into())?;

    //--------Checking project status-------------------------
    if x.project_done != Uint128::zero()
    {
        return Err(ContractError::AlreadyDoneFail{});
    }

    //---------calc project collection---------------------------
    let mut collected = 0;
    let backer_states = x.backer_states.clone();
    for backer in backer_states{
        collected += backer.ust_amount.amount.u128();
    }

    //---------calc total real backed money on smart contract----------------
    //----------map to vec-----------------------
    let all: StdResult<Vec<_>> = PROJECTSTATES.range(deps.storage, None, None, 
        cosmwasm_std::Order::Ascending).collect();
    let all = all.unwrap();

    let mut total_real_backed = 0;
    for x in all{
        let prj = x.1;
        if prj.project_done == Uint128::zero() //exclude done or fail project
        {
            for y in prj.backer_states{
                total_real_backed += y.ust_amount.amount.u128();
            }
        }
    }
    //----------load config and read aust token address-----------------
    let config = CONFIG.load(deps.storage).unwrap();
     
    //--------get aust balance---------------------
    let aust_balance: Cw20BalanceResponse = deps.querier.query_wasm_smart(
        config.aust_token.clone(),
        &Cw20QueryMsg::Balance{
            address: _env.contract.address.to_string(),
        }
    )?;

    //----------calc declaim aust amount---aust*(collcted/total)-----------
    let withdraw_amount = 
        Uint128::from(aust_balance.balance.u128() 
        * collected / total_real_backed);


    let mut msg= Vec::new();

    //----ask aust_token for transfer to anchor martket and execute redeem_stable ----------
    let withdraw = WasmMsg::Execute {
        contract_addr: String::from(config.aust_token),
        msg: to_binary(&Cw20ExecuteMsg::Send {
            contract: config.anchor_market.to_string(),
            msg: to_binary(&Cw20HookMsg::RedeemStable{}).unwrap(), //redeem_stable{}
            amount: withdraw_amount
        }).unwrap(),
        funds: Vec::new()
    };

    msg.push(CosmosMsg::Wasm(withdraw));

    //---------send to backer wallet-------------
    for backer in x.backer_states{
        let backed_ust = backer.ust_amount;
        let send2_backer = BankMsg::Send { 
            to_address: backer.backer_wallet,
            amount: vec![backed_ust] 
        };
        msg.push(CosmosMsg::Bank(send2_backer));
    }
    
    // remove_project(deps, _project_id);
    //-----update project state to FAIL----------------------------

    PROJECTSTATES.update(deps.storage, _project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_done = Uint128::new(2); //fail
            Ok(project)
        }
    })?;

    Ok(Response::new()
    .add_messages(msg)
    .add_attribute("action", "project failed")
    )
}

pub fn try_addproject(
    deps:DepsMut,
    _info: MessageInfo,
    _project_name: String,
    _project_createddate: String,
    _project_description: String,
    _project_teamdescription: String,
    _project_category: String,
    _project_subcategory: String,
    _project_chain:String,
    _project_collected: Uint128,
    _project_deadline: String,
    _project_website: String,
    _project_icon: String,
    _project_email: String,
    _project_whitepaper: String,
    _creator_wallet: String,
) -> Result<Response, ContractError> 
{
    // let res = PROJECTSTATES.may_load(deps.storage, _project_id.u128().into());
    // if res != Ok(None) {//exist
    //     return Err(ContractError::AlreadyRegisteredProject {});
    // }

    // increment id if exists, or return 1
    let id = PROJECT_SEQ.load(deps.storage)?;
    let id = id.checked_add(Uint128::new(1)).unwrap();
    PROJECT_SEQ.save(deps.storage, &id)?;

    // save project state with id
   
    let backer_states = Vec::new();
    let new_project:ProjectState = ProjectState{
        project_id: id,
        project_name: _project_name,
        project_createddate: _project_createddate,
        project_description: _project_description,
        project_teamdescription: _project_teamdescription,
        project_category: _project_category,
        project_subcategory: _project_subcategory,
        project_chain: _project_chain,
        project_collected: _project_collected,
        project_deadline: _project_deadline,
        project_website: _project_website,
        project_icon: _project_icon,
        project_email: _project_email,
        project_whitepaper: _project_whitepaper,
        creator_wallet: _creator_wallet,
        project_needback: true,
        project_done: Uint128::zero(),
        backer_states: backer_states,
    };
        
    PROJECTSTATES.save(deps.storage, id.u128().into(), &new_project)?;
    Ok(Response::new()
        .add_attribute("action", "add project"))
}

pub fn try_back2project(
    deps:DepsMut, 
    info: MessageInfo,
    _project_id:Uint128, 
    _backer_wallet:String
) -> Result<Response, ContractError> 
{
    //-------check project exist
    let res = PROJECTSTATES.may_load(deps.storage, _project_id.u128().into());
    if res == Ok(None) { //not exist
        return Err(ContractError::NotRegisteredProject {});
    }
    //--------Get project info----------------------------
    let mut x = PROJECTSTATES.load(deps.storage, _project_id.u128().into())?;
    if x.project_needback == false {
        return Err(ContractError::AlreadyCollected{});
    }

    //--------check sufficient back--------------------
    let fee:u128 = 100000000;
    if info.funds.is_empty() || info.funds[0].amount.u128() < fee{
        return Err(ContractError::NeedCoin{});
    }
 
    let fund = info.funds[0].clone();
 
    //---------check collection---------------------------
    let mut needback = true;
    let mut collected = 0;
    let backer_states = x.backer_states.clone();

    for backer in backer_states{
        collected += backer.ust_amount.amount.u128();
    }
    if collected + fund.amount.u128() >= (x.project_collected.u128()*1000000){
        needback = false;
    }
    //-------------add new backer to PROJECTSTATE--------------
    let mut fund_real_back = fund.clone();  
    let amount_real_back = fund_real_back.amount.u128() * 100 / 105;
    fund_real_back.amount = Uint128::new(amount_real_back);

    let new_baker:BackerState = BackerState{
        backer_wallet:_backer_wallet,
        ust_amount: fund_real_back.clone(),
        aust_amount: Coin::new(0, "aust")
    };

    x.backer_states.push(new_baker);
     
    PROJECTSTATES.update(deps.storage, _project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_needback = needback;
            project.backer_states = x.backer_states.clone();
            Ok(project)
        }
    })?;

    //----------load config and read anchor market address-----------------
    let config = CONFIG.load(deps.storage).unwrap();
    let anchormarket = config.anchor_market;

    
    //----------deposite to anchor market------------------------
    let deposite_project = WasmMsg::Execute {
            contract_addr: String::from(anchormarket),
            msg: to_binary(&AnchorMarket::DepositStable {}).unwrap(),
            funds: vec![fund_real_back]
    };

    //---------send to Wefund with 5/105--------------------
    let mut fund_wefund = fund.clone();
    let amount_wefund = (fund_wefund.amount.u128() * 5 / 105) - 4000000;
    fund_wefund.amount = Uint128::new(amount_wefund);
    let bank_wefund = BankMsg::Send { 
        to_address: config.wefund.to_string(),
        amount: vec![fund_wefund] 
    };

    Ok(Response::new()
    .add_messages(vec![
        CosmosMsg::Wasm(deposite_project),
        CosmosMsg::Bank(bank_wefund)])
    .add_attribute("action", "back to project")
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBalance{ wallet } => to_binary(&query_balance(deps, _env, wallet)?),
        QueryMsg::GetConfig{ } => to_binary(&query_getconfig(deps)?),
        QueryMsg::GetAllProject{ } => to_binary(&query_allproject(deps)?),
        QueryMsg::GetProject{ project_id } => to_binary(&query_project(deps, project_id)?),
        QueryMsg::GetBacker{ project_id } => to_binary(&query_backer(deps, project_id)?),
    }
}

fn query_balance(deps:Deps, _env:Env, wallet:String) -> StdResult<AllBalanceResponse>{

    // let uusd_denom = String::from("uusd");
    let mut balance: AllBalanceResponse = deps.querier.query(
        &QueryRequest::Bank(BankQuery::AllBalances {
            address: wallet.clone(),
        }
    ))?;

    let config = CONFIG.load(deps.storage).unwrap();

    let aust_balance: Cw20BalanceResponse = deps.querier.query_wasm_smart(
        config.aust_token,
        &Cw20QueryMsg::Balance{
            address: wallet,
        }
    )?;
    balance.amount.push(Coin::new(aust_balance.balance.u128(), "aust"));

    Ok(balance)
}
fn query_getconfig(deps:Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage).unwrap();
    Ok(config)
}
fn query_allproject(deps:Deps) -> StdResult<Vec<ProjectState>> {
    let all: StdResult<Vec<_>> = PROJECTSTATES.range(deps.storage, None, None, 
        cosmwasm_std::Order::Ascending).collect();
    let all = all.unwrap();

    let mut all_project:Vec<ProjectState> = Vec::new();
    for x in all{
        all_project.push(x.1);
    }
    Ok(all_project)
}
fn query_backer(deps:Deps, id:Uint128) -> StdResult<Vec<BackerState>>{
    let x = PROJECTSTATES.load(deps.storage, id.u128().into())?;
    Ok(x.backer_states)
}
fn query_project(deps:Deps, id:Uint128) -> StdResult<ProjectState>{
    let x = PROJECTSTATES.load(deps.storage, id.u128().into())?;
    
    Ok(x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, 
        MOCK_CONTRACT_ADDR, MockQuerier};
    use cosmwasm_std::{from_binary, Addr, CosmosMsg, WasmMsg,
        BankQuery, BalanceResponse, Coin };
    #[test]
    fn workflow(){
        let mut deps = mock_dependencies(&[]);
        
        let msg = InstantiateMsg{
            admin: Some(String::from("admin")),
            wefund: Some(String::from("Wefund")),
            anchor_market: Some(String::from("market")),
            aust_token: Some(String::from("ETH"))
        };

        let info = mock_info("creator", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

//add project        

       let msg = ExecuteMsg::AddProject{
	        creator_wallet: String::from("terra1emwyg68n0wtglz8ex2n2728fnfzca9xkdc4aka"),
            project_description: String::from("demo1"),
            project_category: String::from("Charity"),
            project_collected: Uint128::new(5000000000),
            project_chain: String::from("Terra"),
            project_email: String::from("deme1@gmail.com"),
            project_name: String::from("demo1"),
            project_website: String::from("https://demo1"),
            project_createddate: String::from("20211223"),
            project_icon: String::from("icon1"),
            project_deadline: String::from("20220130"),
            project_subcategory: String::from("gaming"),
            project_teamdescription: String::from("demo1"),
            project_whitepaper: String::from("whitepaper")
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        // assert_eq!(res.messages.len(), 0);
println!("{:?}", res);

//add project        

let msg = ExecuteMsg::AddProject{
    creator_wallet: String::from("anyone"),
    project_description: String::from("demo2"),
    project_category: String::from("terra"),
    project_collected: Uint128::new(5000000000),
    project_chain: String::from("Terra"),
    project_email: String::from("deme2@gmail.com"),
    project_name: String::from("demo2"),
    project_website: String::from("https://demo1"),
    project_createddate: String::from("20211223"),
    project_icon: String::from("icon2"),
    project_deadline: String::from("20220130"),
    project_subcategory: String::from("gaming"),
    project_teamdescription: String::from("demo2"),
    project_whitepaper: String::from("whitepaper")
};
let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
// assert_eq!(res.messages.len(), 0);
println!("{:?}", res);

//balance

// back 2 projct
        let info = mock_info("backer1", &[Coin::new(105000000, "uusd")]);
        let msg = ExecuteMsg::Back2Project{
            project_id: Uint128::new(1),
            backer_wallet: String::from("backer1"),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
println!("back2project:{:?}", res);
        
        let info = mock_info("backer2", &[Coin::new(210000000, "uusd")]);
        let msg = ExecuteMsg::Back2Project{
            project_id: Uint128::new(1),
            backer_wallet: String::from("backer2"),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
println!("back2project:{:?}", res);

//-Get Project-----------------
        let msg = QueryMsg::GetAllProject{};
        let allproject = query(deps.as_ref(), mock_env(), msg).unwrap();

        let res:Vec<ProjectState> = from_binary(&allproject).unwrap();
        println!("allproject {:?}", res );
//-Get Config-------------            
        let msg = QueryMsg::GetConfig{};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();

        let config:Config= from_binary(&res).unwrap();
        println!("Config = {:?}", config);
//-Complete project--------------------------
        // let msg = ExecuteMsg::CompleteProject{project_id:Uint128::new(1)};
        // let res = execute(deps.as_mut(), mock_env(), info, msg);

//-Get project1 Balance-------------------
        // let msg = QueryMsg::GetBalance{ wallet: String::from("wefund")};
        // let balance = query(deps.as_ref(), mock_env(), msg).unwrap();

        // let res:AllBalanceResponse = from_binary(&balance).unwrap();
        // println!("wefund Balance {:?}", res );
//-Get wefund Balance-------------------
        // let msg = QueryMsg::GetBalance{ wallet: String::from("market")};
        // let balance = query(deps.as_ref(), mock_env(), msg).unwrap();

        // let res:AllBalanceResponse = from_binary(&balance).unwrap();
        // println!("market Balance {:?}", res );

//-Remove Project-------------------------
        let info = mock_info("admin", &[Coin::new(105000000, "uusd")]);
        let msg = ExecuteMsg::RemoveProject{project_id:Uint128::new(1)};
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

//-Get Project-----------------
        let msg = QueryMsg::GetAllProject{};
        let allproject = query(deps.as_ref(), mock_env(), msg).unwrap();

        let res:Vec<ProjectState> = from_binary(&allproject).unwrap();
        println!("allproject {:?}", res );
    }
}

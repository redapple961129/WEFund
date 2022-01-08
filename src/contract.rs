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
use crate::msg::{ExecuteMsg, QueryMsg, InstantiateMsg};
use crate::state::{Config, CONFIG, PROJECTSTATES, ProjectState, BackerState,
        PROJECT_SEQ, COMMUNITY, Milestone, Vote, save_projectstate};

use crate::market::{ExecuteMsg as AnchorMarket, Cw20HookMsg};                    

// version info for migration info
const CONTRACT_NAME: &str = "WEFUND";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const ust: u128 = 1000000; //ust unit

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
    COMMUNITY.save(deps.storage, &Vec::new())?;

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
            project_milestones,
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
                project_milestones,
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

        ExecuteMsg::AddCommunitymember{wallet} =>
            try_addcommunitymember(deps, info, wallet),

        ExecuteMsg::RemoveCommunitymember{wallet} =>
            try_removecommunitymember(deps, info, wallet),

        ExecuteMsg::WeFundApprove{project_id} =>
            try_wefundapprove(deps, info, project_id),

        ExecuteMsg::SetCommunityVote{project_id, wallet, voted} =>
            try_setcommunityvote(deps, project_id, wallet, voted),

        ExecuteMsg::SetMilestoneVote{project_id, wallet, voted} =>
            try_setmilestonevote(deps, project_id, wallet, voted),

        ExecuteMsg::ReleaseMilestone{project_id} =>
            try_releasemilestone(deps, project_id),
    }
}
pub fn try_releasemilestone(
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
    if x.project_status != Uint128::zero()
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
        if prj.project_status == Uint128::zero() //exclude done or fail project
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
        to_address: x.creator_wallet.to_string(),
        amount: vec![ust_collected] 
    };

    // remove_project(deps, _project_id);
    //-----update project state to DONE----------------------------
    PROJECTSTATES.update(deps.storage, _project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_status = Uint128::new(1); //done
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
pub fn try_setmilestonevote(deps: DepsMut, project_id: Uint128, wallet: String, voted: bool)
    -> Result<Response, ContractError>
{
    let mut x:ProjectState = PROJECTSTATES.load(deps.storage, project_id.u128().into())?;
    
    //-------check project status-------------------
    if x.project_status != Uint128::new(3) { //only releasing status
        return Err(ContractError::NotCorrectStatus{status:x.project_status});
    }

    let wallet = deps.api.addr_validate(&wallet).unwrap();
    let step = x.project_milestonestep.u128() as usize;

    if x.milestone_states[step].milestone_status != Uint128::zero(){//only voting status
        return Err(ContractError::NotCorrectMilestoneStatus{
            step:step, status:x.milestone_states[step].milestone_status 
        })
    }

    //------set vot status--------------------
    let index = x.milestone_states[step].milestone_votes.iter().position(|x|x.wallet == wallet).unwrap();
    x.milestone_states[step].milestone_votes[index].voted = voted;

    //------check all voted-----------------
    let mut all_voted = true;
    for vote in x.milestone_states[step].milestone_votes.clone(){
        all_voted = all_voted & vote.voted;
    }
    if all_voted{
        x.milestone_states[step].milestone_status = Uint128::new(1); //switch to releasing status
        //-----------------release function---------------

        x.milestone_states[step].milestone_status = Uint128::new(2); //switch to released status
        x.project_milestonestep += Uint128::new(1); //switch to next milestone step
        
        //-----------check milestone done---------------------
        if x.project_milestonestep > Uint128::new(x.milestone_states.len() as u128){
            x.project_status = Uint128::new(4); //switch to project done status
        }
    }

    //-------update-------------------------
    PROJECTSTATES.update(deps.storage, project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.milestone_states = x.milestone_states;
            project.project_milestonestep = x.project_milestonestep;
            project.project_status = x.project_status;
            Ok(project)
        }
    })?;

    Ok(Response::new()
    .add_attribute("action", "Set milestone vote")
    )
}

pub fn try_setcommunityvote(deps: DepsMut, project_id: Uint128, wallet: String, voted: bool)
    -> Result<Response, ContractError>
{
    let mut x:ProjectState = PROJECTSTATES.load(deps.storage, project_id.u128().into())?;
    
    //-------check project status-------------------
    if x.project_status != Uint128::new(1) { //only community voting status
        return Err(ContractError::NotCorrectStatus{status:x.project_status});
    }
    let wallet = deps.api.addr_validate(&wallet).unwrap();
    let index = x.community_votes.iter().position(|x|x.wallet == wallet).unwrap();

    //------set vot status--------------------
    x.community_votes[index].voted = voted;

    //------check all voted-----------------
    let mut all_voted = true;
    for vote in x.community_votes.clone(){
        all_voted = all_voted & vote.voted;
    }
    if all_voted{
        x.project_status = Uint128::new(2); //switch to fundrasing status
    }

    //-------update-------------------------
    PROJECTSTATES.update(deps.storage, project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_status = x.project_status;
            project.community_votes = x.community_votes;
            Ok(project)
        }
    })?;

    Ok(Response::new()
    .add_attribute("action", "Set community vote")
    )
}

pub fn try_wefundapprove(deps: DepsMut, info:MessageInfo, project_id: Uint128)
    ->Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner{
        return Err(ContractError::Unauthorized{});
    }

    let mut x:ProjectState = PROJECTSTATES.load(deps.storage, project_id.u128().into())?;
    
    //-------check project status-------------------
    if x.project_status != Uint128::zero() { //only wefund approve status
        return Err(ContractError::NotCorrectStatus{status:x.project_status});
    }
    x.project_status = Uint128::new(1); //switch to community voting status

    PROJECTSTATES.update(deps.storage, project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_status = x.project_status;
            Ok(project)
        }
    })?;

    Ok(Response::new()
    .add_attribute("action", "Wefund Approve")
    )
}

pub fn try_removecommunitymember(deps:DepsMut, info:MessageInfo, wallet: String)
    -> Result<Response, ContractError>
{
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner{
        return Err(ContractError::Unauthorized{});
    }

    let wallet = deps.api.addr_validate(&wallet).unwrap();

    let mut community = COMMUNITY.load(deps.storage).unwrap();
    let res = community.iter().find(|&x| x == &wallet);
    if res == None {
        return Err(ContractError::NotRegisteredCommunity{});
    }

    community.retain(|x|x != &wallet);
    COMMUNITY.save(deps.storage, &community)?;

    Ok(Response::new()
    .add_attribute("action", "remove community member")
    )
}

pub fn try_addcommunitymember(deps:DepsMut, info:MessageInfo, wallet: String)
    -> Result<Response, ContractError>
{
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner{
        return Err(ContractError::Unauthorized{});
    }

    let wallet = deps.api.addr_validate(&wallet).unwrap();

    let mut community = COMMUNITY.load(deps.storage).unwrap();
    let res = community.iter().find(|&x| x == &wallet);
    if res != None {
        return Err(ContractError::AlreadyRegisteredCommunity{});
    }

    community.push(wallet);
    COMMUNITY.save(deps.storage, &community)?;

    Ok(Response::new()
    .add_attribute("action", "add community member")
    )
}
pub fn try_transferallcoins(deps:DepsMut, _env:Env, info:MessageInfo, wallet:String)
    -> Result<Response, ContractError>
{
    //-----------check owner--------------------------
    let config = CONFIG.load(deps.storage).unwrap();
    if info.sender != config.owner{
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
    if info.sender != config.owner {
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
    if x.project_status != Uint128::zero()
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
        if prj.project_status == Uint128::zero() //exclude done or fail project
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
        to_address: x.creator_wallet.to_string(),
        amount: vec![ust_collected] 
    };

    // remove_project(deps, _project_id);
    //-----update project state to DONE----------------------------
    PROJECTSTATES.update(deps.storage, _project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_status = Uint128::new(1); //done
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
    if x.project_status != Uint128::zero()
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
        if prj.project_status == Uint128::zero() //exclude done or fail project
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
            to_address: backer.backer_wallet.to_string(),
            amount: vec![backed_ust] 
        };
        msg.push(CosmosMsg::Bank(send2_backer));
    }
    
    // remove_project(deps, _project_id);
    //-----update project state to FAIL----------------------------

    PROJECTSTATES.update(deps.storage, _project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_status = Uint128::new(2); //fail
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
    _project_milestones: Vec<Milestone>,
) -> Result<Response, ContractError> 
{
    let community = COMMUNITY.load(deps.storage).unwrap();
    let mut community_votes = Vec::new();
    for x in community{
        community_votes.push(
            Vote{wallet:x, voted: false}
        );
    }

    let new_project:ProjectState = ProjectState{
        project_name: _project_name,
        project_createddate: _project_createddate,
        project_description: _project_description,
        project_teamdescription: _project_teamdescription,
        project_category: _project_category,
        project_subcategory: _project_subcategory,
        project_chain: _project_chain,
        project_deadline: _project_deadline,
        project_website: _project_website,
        project_icon: _project_icon,
        project_email: _project_email,
        project_whitepaper: _project_whitepaper,

        project_id: Uint128::zero(), //auto increment
        creator_wallet: deps.api.addr_validate(&_creator_wallet).unwrap(),
        project_collected: _project_collected,
        project_status: Uint128::zero(), //wefund voting status

        backerbacked_amount: Uint128::zero(),
        communitybacked_amount: Uint128::zero(),

        backer_states: Vec::new(),
        communitybacker_states: Vec::new(),

        milestone_states: _project_milestones,
        project_milestonestep: Uint128::zero(), //first milestonestep

        community_votes: community_votes,
    };

    save_projectstate(deps, &new_project)?;
    Ok(Response::new()
        .add_attribute("action", "add project"))
}

pub fn try_back2project(
    deps:DepsMut, 
    info: MessageInfo,
    project_id:Uint128, 
    backer_wallet:String
) -> Result<Response, ContractError> 
{
    //-------check project exist-----------------------------------
    let res = PROJECTSTATES.may_load(deps.storage, project_id.u128().into());
    if res == Ok(None) { //not exist
        return Err(ContractError::NotRegisteredProject {});
    }
    //--------Get project info------------------------------------
    let mut x = PROJECTSTATES.load(deps.storage, project_id.u128().into())?;
    if x.project_status != Uint128::new(2){//only fundrasing status
        return Err(ContractError::NotCorrectStatus{status: x.project_status});
    }

    //--------check sufficient back--------------------
    let fee:u128 = 4 * ust;
    if info.funds.is_empty() || info.funds[0].amount.u128() < 6 * ust{
        return Err(ContractError::NeedCoin{});
    }
 
    let fund = info.funds[0].clone();
    let mut fund_real_back = fund.clone();
    let mut fund_wefund = fund.clone();
    //--------calc amount to desposit and to wefund
    if fund.amount.u128() >= 100 * ust{
        fund_real_back.amount = Uint128::new(fund.amount.u128() * 100 / 105);
        fund_wefund.amount = Uint128::new((fund.amount.u128() * 5 / 105) - fee);
    } else {
        fund_real_back.amount = Uint128::new(fund.amount.u128() - 5 * ust);
        fund_wefund.amount = Uint128::new(1 * ust);
    }

    let backer_wallet = deps.api.addr_validate(&backer_wallet).unwrap();

    //--------check community and calc backed amount----------------
    let community = COMMUNITY.load(deps.storage)?;
    let is_community = community.iter().find(|&x| x == &backer_wallet);
    let collected = Uint128::new(x.project_collected.u128() / 2 * ust);

    if is_community != None { //community backer
        if x.communitybacked_amount >= collected{
            return Err(ContractError::AlreadyCollected{});
        }
        x.communitybacked_amount += fund_real_back.amount;
    } else { //only backer
        if x.backerbacked_amount >= collected{
            return Err(ContractError::AlreadyCollected{});
        }
        x.backerbacked_amount += fund_real_back.amount;
    }
    //------push to new backer------------------
    let new_baker:BackerState = BackerState{
        backer_wallet: backer_wallet,
        ust_amount: fund_real_back.clone(),
        aust_amount: Coin::new(0, "aust")
    };
    if is_community != None {//community backer
        x.communitybacker_states.push(new_baker);
    } else {
        x.backer_states.push(new_baker);
    }

    //------check needback-----------------
    let mut communitybacker_needback = true;
    let mut backer_needback = true;

    if x.communitybacked_amount  >= collected{
        communitybacker_needback = false;
    }
    if x.backerbacked_amount  >= collected{
        backer_needback = false;
    }

    //---------check collection and switch to releasing status---------
    if communitybacker_needback == false && backer_needback == false{
        x.project_status = Uint128::new(3); //releasing

        //------add milestone votes in every milestone---------------
        let mut milestone_votes = Vec::new();
        for backer in x.backer_states.clone(){
            milestone_votes.push(
                Vote{ wallet: backer.backer_wallet, voted: false }
            );
        }
        //-----add wefund vote------------------
        let config = CONFIG.load(deps.storage)?;
        milestone_votes.push(
            Vote{ wallet: config.owner, voted: true}
        );

        for i in 0..x.milestone_states.len(){
            x.milestone_states[i].milestone_votes = milestone_votes.clone();
        }
    }

    PROJECTSTATES.update(deps.storage, project_id.u128().into(), |op| match op {
        None => Err(ContractError::NotRegisteredProject {}),
        Some(mut project) => {
            project.project_status = x.project_status;
            project.communitybacked_amount = x.communitybacked_amount;
            project.backerbacked_amount = x.backerbacked_amount;
            project.backer_states = x.backer_states;
            project.communitybacker_states = x.communitybacker_states;
            
            if x.project_status == Uint128::new(3){//only on switching releasing status
                project.milestone_states = x.milestone_states;
            }
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
        QueryMsg::GetCommunitymembers{ } => to_binary(&query_communitymembers(deps)?),
    }
}

fn query_communitymembers(deps:Deps) -> StdResult<Vec<Addr>>{
    let community = COMMUNITY.load(deps.storage).unwrap();
    Ok(community)
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
    use crate::state::{Milestone};

    #[test]
    fn workflow(){
        let mut deps = mock_dependencies(&[]);
        
        let msg = InstantiateMsg{
            admin: Some(String::from("admin")),
            wefund: Some(String::from("Wefund")),
            anchor_market: Some(String::from("market")),
            aust_token: Some(String::from("ETH"))
        };
//instantiate
        let info = mock_info("admin", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
//add community member
        let msg = ExecuteMsg::AddCommunitymember{
            wallet: String::from("community1")
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        println!("Add community member{:?}", res);
        //-------------------------------
        let msg = ExecuteMsg::AddCommunitymember{
            wallet: String::from("community2")
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        println!("Add community member{:?}", res);

        // let msg = ExecuteMsg::RemoveCommunitymember{
        //     wallet: String::from("community3")
        // };
        // let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        // println!("Remove community member{:?}", res);
//add project        
        let milestone1 = Milestone{
            milestone_step: Uint128::new(0),
            milestone_name: String::from("milestone1"),
            milestone_description: String::from("mileston1"),
            milestone_startdate: String::from("startdate"),
            milestone_enddate: String::from("enddate"),
            milestone_amount: Uint128::new(100),
            milestone_status: Uint128::new(0),
            milestone_votes: Vec::new()
        };
        let milestone2 = Milestone{
            milestone_step: Uint128::new(1),
            milestone_name: String::from("milestone2"),
            milestone_description: String::from("mileston2"),
            milestone_startdate: String::from("startdate"),
            milestone_enddate: String::from("enddate"),
            milestone_amount: Uint128::new(200),
            milestone_status: Uint128::new(0),
            milestone_votes: Vec::new()
        };
        let milestone_states = vec![milestone1, milestone2];
        let msg = ExecuteMsg::AddProject{
	        creator_wallet: String::from("terra1emwyg68n0wtglz8ex2n2728fnfzca9xkdc4aka"),
            project_description: String::from("demo1"),
            project_category: String::from("Charity"),
            project_collected: Uint128::new(300),
            project_chain: String::from("Terra"),
            project_email: String::from("deme1@gmail.com"),
            project_name: String::from("demo1"),
            project_website: String::from("https://demo1"),
            project_createddate: String::from("20211223"),
            project_icon: String::from("icon1"),
            project_deadline: String::from("20220130"),
            project_subcategory: String::from("gaming"),
            project_teamdescription: String::from("demo1"),
            project_whitepaper: String::from("whitepaper"),
            project_milestones: milestone_states,
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        // assert_eq!(res.messages.len(), 0);
        println!("{:?}", res);

//add project        

        let msg = ExecuteMsg::AddProject{
            creator_wallet: String::from("anyone"),
            project_description: String::from("demo2"),
            project_category: String::from("terra"),
            project_collected: Uint128::new(300),
            project_chain: String::from("Terra"),
            project_email: String::from("deme2@gmail.com"),
            project_name: String::from("demo2"),
            project_website: String::from("https://demo1"),
            project_createddate: String::from("20211223"),
            project_icon: String::from("icon2"),
            project_deadline: String::from("20220130"),
            project_subcategory: String::from("gaming"),
            project_teamdescription: String::from("demo2"),
            project_whitepaper: String::from("whitepaper"),
            project_milestones: Vec::new(),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        // assert_eq!(res.messages.len(), 0);
        println!("{:?}", res);

//Wefund Approve
        let info = mock_info("admin", &[]);
        let msg = ExecuteMsg::WeFundApprove{
            project_id: Uint128::new(1),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        println!("WeFund Approve: {:?}", res);

        let info = mock_info("admin", &[]);
        let msg = ExecuteMsg::WeFundApprove{
            project_id: Uint128::new(2),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        println!("WeFund Approve: {:?}", res);
//Set community vote
        let info = mock_info("community1", &[]);
        let msg = ExecuteMsg::SetCommunityVote{
            project_id: Uint128::new(1),
            wallet: String::from("community1"),
            voted: true
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        println!("Set Community vote: {:?}", res);

        let info = mock_info("community2", &[]);
        let msg = ExecuteMsg::SetCommunityVote{
            project_id: Uint128::new(1),
            wallet: String::from("community2"),
            voted: true
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();
        println!("Set Community vote: {:?}", res);
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

        let info = mock_info("community1", &[Coin::new(210000000, "uusd")]);
        let msg = ExecuteMsg::Back2Project{
            project_id: Uint128::new(1),
            backer_wallet: String::from("community1"),
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

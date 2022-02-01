use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128, Coin, StdResult, DepsMut};
use cw_storage_plus::{Item, Map, U128Key};
//------------Config---------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub wefund: Addr,
    pub anchor_market: Addr,
    pub aust_token: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

//-------------backer states---------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BackerState{
    pub backer_wallet: Addr,
    pub ust_amount: Coin,
    pub aust_amount: Coin,
    pub otherchain: String,
    pub otherchain_wallet: String,
}
//--------------Vote---------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vote{
    pub wallet: Addr,
    pub voted: bool,
}

//--------------Milestone---------------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Milestone{
    pub milestone_step: Uint128,
    pub milestone_name: String,
    pub milestone_description: String,
    pub milestone_startdate: String,
    pub milestone_enddate: String,
    pub milestone_amount: Uint128,
    pub milestone_status: Uint128, //0:voting, 1:releasing 2:released
    pub milestone_votes: Vec<Vote>,
}

//------------ project state--------------------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ProjectState{
//---------mata data----------------------------------------------------------
    pub project_name: String,
    pub project_createddate: String,
    pub project_description: String,
    pub project_teamdescription: String,
    pub project_category: String,
    pub project_subcategory: String,
    pub project_chain: String,
    pub project_deadline: String,
    pub project_website: String,
    pub project_icon: String,
    pub project_email: String,
    pub project_whitepaper: String,
//------------------------------------------------------------------------------
    pub project_id: Uint128,
    pub creator_wallet: Addr,
    pub project_collected: Uint128,

    //0:wefund voting 1:community voting 2:fundrasing 3:releasing 4:done 5:fail
    pub project_status: Uint128, 

    pub backerbacked_amount: Uint128,
    pub communitybacked_amount: Uint128,
//---------backer states for 50% of collected------------------------    
    pub backer_states: Vec<BackerState>,

//---------community backer states for 50% of collected---------------
    pub communitybacker_states: Vec<BackerState>,

//----------milestone states-----------------------------------------
    pub milestone_states: Vec<Milestone>,
    pub project_milestonestep: Uint128, 
//---------community votes--------------------------------------------
    pub community_votes: Vec<Vote>,
    pub community_vote_deadline: Uint128,
}
pub const PROJECT_SEQ: Item<Uint128> = Item::new("prj_seq");
pub const PROJECTSTATES: Map<U128Key, ProjectState> = Map::new("prj");

pub fn save_projectstate(deps: DepsMut, _prj: &ProjectState) 
    -> StdResult<()> 
{
    // increment id if exists, or return 1
    let id = PROJECT_SEQ.load(deps.storage)?;
    let id = id.checked_add(Uint128::new(1))?;
    PROJECT_SEQ.save(deps.storage, &id)?;

    // save pot with id
    let mut project = _prj.clone();
    project.project_id = id.clone();
    PROJECTSTATES.save(deps.storage, id.u128().into(), &project)
}

//------------community array------------------------------------------------
pub const COMMUNITY: Item<Vec<Addr>> = Item::new("community");
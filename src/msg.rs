use cosmwasm_std::{Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::state::{Milestone};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub wefund: Option<String>,
    pub anchor_market: Option<String>,
    pub aust_token: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetConfig { admin:Option<String>,  wefund: Option<String>, 
        anchor_market: Option<String>, aust_token:Option<String> },
    AddProject { 
        project_name: String,
        project_createddate: String,
        project_description: String,
        project_teamdescription: String,
        project_category: String,
        project_subcategory: String,
        project_chain:String,
        project_collected: Uint128,
        project_deadline: String,
        project_website: String,
        project_icon: String,
        project_email: String,
        project_whitepaper: String,
        creator_wallet: String,
        project_milestones: Vec<Milestone>,
    },
    RemoveProject{project_id: Uint128 },

    Back2Project { project_id: Uint128, backer_wallet: String},

    CompleteProject{ project_id: Uint128 },
    FailProject{project_id: Uint128 },

    TransferAllCoins{wallet: String},

    AddCommunitymember{wallet: String},
    RemoveCommunitymember{wallet: String},

    WefundApprove{project_id:Uint128, deadline:Uint128},
    SetCommunityVote{project_id: Uint128, wallet: String, voted: bool},

    SetMilestoneVote{project_id: Uint128, wallet:String, voted: bool},

    ReleaseMilestone{project_id: Uint128},

    SetProjectStatus{project_id: Uint128, status: Uint128}
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig{},
    GetAllProject{},
    GetProject { project_id:Uint128 },
    GetBacker{ project_id:Uint128},
    GetBalance{ wallet:String },
    GetCommunitymembers{},
}


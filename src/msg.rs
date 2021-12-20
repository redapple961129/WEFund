use cosmwasm_std::{Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    AddProject { project_name: String, project_wallet: String, 
        project_collected: Uint128, creator_wallet: String , 
        project_website: String, project_about: String,
        project_email: String, project_ecosystem:String,
        project_category:String},
    Back2Project { project_id: Uint128, backer_wallet: String},
    CompleteProject{ project_id:Uint128 },
    FailProject{project_id:Uint128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig{},
    GetAllProject{},
    GetProject { project_id:Uint128 },
    GetBacker{ project_id:Uint128},
    GetBalance{ wallet:String }
}


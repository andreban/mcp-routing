use serde::{Deserialize, Serialize};

use crate::types::jsonrpc::JsonRpcResultResponse;

pub type ListToolsResultResponse = JsonRpcResultResponse<ListToolsResult>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListToolsResult {}

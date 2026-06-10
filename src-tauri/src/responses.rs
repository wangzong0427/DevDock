use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathStatusResponse {
    pub(crate) state: String,
    pub(crate) bin_dir: String,
    pub(crate) message: String,
    pub(crate) suggested_command: Option<String>,
    pub(crate) paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisteredCommandResponse {
    pub(crate) name: String,
    pub(crate) script_path: String,
    pub(crate) entry_path: String,
    pub(crate) entry_type: String,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandRunResultResponse {
    pub(crate) command_name: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandRunStartedResponse {
    pub(crate) command_name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandRunFailedResponse {
    pub(crate) command_name: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandOutputChunkResponse {
    pub(crate) command_name: String,
    pub(crate) stream: String,
    pub(crate) text: String,
}

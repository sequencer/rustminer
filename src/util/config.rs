use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub pool: Vec<Pool>,
    pub client: Client,
}

#[derive(Deserialize, Debug)]
pub struct Pool {
    pub addr: String,
    pub user: String,
    pub pass: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Client {
    pub user_agent: Option<String>,
    pub version_rolling: VersionRolling,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct VersionRolling {
    pub mask: String,
    pub min_bit_count: Option<u8>,
}

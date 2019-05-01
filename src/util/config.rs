use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub pool: Vec<Pool>,
    pub board: Board,
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub enabled: Vec<u16>,
    pub default: BoardSetting,
    pub _0: Option<BoardSetting>,
    pub _1: Option<BoardSetting>,
    pub _2: Option<BoardSetting>,
    pub _3: Option<BoardSetting>,
    pub _4: Option<BoardSetting>,
    pub _5: Option<BoardSetting>,
    pub _6: Option<BoardSetting>,
    pub _7: Option<BoardSetting>,
}

impl Board {
    pub fn get_setting(&self, id: u16) -> (f32, u32) {
        let mut setting = (8.6, 108);
        if let Some(custom) = match id {
            0 => &self._0,
            1 => &self._1,
            2 => &self._2,
            3 => &self._3,
            4 => &self._4,
            5 => &self._5,
            6 => &self._6,
            7 => &self._7,
            _ => unreachable!(),
        } {
            if let Some(voltage) = custom.voltage {
                setting.0 = voltage;
            } else if let Some(voltage) = self.default.voltage {
                setting.0 = voltage;
            }

            if let Some(param) = custom.param {
                setting.1 = param;
            } else if let Some(param) = self.default.param {
                setting.1 = param;
            }
        }
        setting
    }
}

#[derive(Deserialize, Debug)]
pub struct BoardSetting {
    pub voltage: Option<f32>,
    pub param: Option<u32>,
}

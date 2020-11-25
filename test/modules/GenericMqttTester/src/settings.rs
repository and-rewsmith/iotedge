use std::env;
use std::str::FromStr;

use anyhow::Result;
use strum_macros::EnumString;

const TEST_SCENARIO_ENV: &str = "TEST_SCENARIO";
// const EXPIRY_TIME_ENV: &str = "";
// const VALIDATION_TIME_ENV: &str = "";
// const TEST_START_DELAY_ENV: &str = "";
// const MESSAGE_SEND_FREQ_ENV: &str = "";

#[derive(Clone, EnumString)]
pub enum TestScenario {
    Receive,
    Send,
}

#[derive(Clone)]
pub struct Settings {
    test_scenario: TestScenario,
}

impl Settings {
    pub fn merge_env(mut self) -> Result<Self> {
        self.test_scenario = TestScenario::from_str(env::var(TEST_SCENARIO_ENV)?.as_str())?;
        Ok(self)
    }

    pub fn test_scenario(&self) -> &TestScenario {
        &self.test_scenario
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            test_scenario: TestScenario::Send,
        }
    }
}

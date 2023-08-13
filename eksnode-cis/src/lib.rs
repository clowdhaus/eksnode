mod eks;

use std::fmt;

#[derive(Default)]
pub enum CheckResult {
  Pass,
  Fail,
  NotApplicable,
  #[default]
  NotChecked,
}

impl fmt::Display for CheckResult {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      CheckResult::Pass => write!(f, "PASS"),
      CheckResult::Fail => write!(f, "FAIL"),
      CheckResult::NotApplicable => write!(f, "NOT-APPLICABLE"),
      CheckResult::NotChecked => write!(f, "NOT-CHECKED"),
    }
  }
}

#[derive(Default)]
pub struct Check {
  pub id: String,
  pub text: String,
  pub remediation: String,
  pub result: CheckResult,
  pub expected_value: Option<String>,
  pub actual_value: Option<String>,
}

impl Check {
  pub fn new(id: &str, text: &str, remediation: &str) -> Self {
    Self {
      id: id.into(),
      text: text.into(),
      remediation: remediation.into(),
      ..Check::default()
    }
  }
}

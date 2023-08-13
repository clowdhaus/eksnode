use anyhow::Result;

use crate::{Check, CheckResult};
pub struct Node {
  pub checks: Vec<crate::Check>,
}

impl Node {
  pub fn new(&mut self) -> Self {
    Self { checks: Vec::new() }
  }
}

async fn check_3_1_1() -> Result<Check> {
  let mut check = Check::new(
    "3.1.1",
    "Ensure that the kubeconfig file permissions are set to 644 or more restrictive (Manual)",
    "Run the below command (based on the file location on your system) on the each worker node.
          For example,
          chmod 644 $kubeletkubeconfig",
  );

  check.result = CheckResult::Fail;

  Ok(check)
}

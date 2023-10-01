use std::fmt;

use anyhow::{anyhow, Result};
use tracing::info;

use crate::utils::cmd_exec;

enum NvidiaGpuClock {
  Graphics,
  Memory,
}

impl fmt::Display for NvidiaGpuClock {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Graphics => write!(f, "graphics"),
      Self::Memory => write!(f, "memory"),
    }
  }
}

fn get_nvidia_max_clock(clock_type: &NvidiaGpuClock) -> Result<i32> {
  let output = cmd_exec(
    "nvidia-smi",
    vec![
      &format!("--query-supported-clocks={}", &clock_type.to_string()),
      "--format=csv",
    ],
  )?;

  let clock_speeds = output
    .stdout
    .lines()
    .filter_map(|line| {
      let mut clock = line.split_whitespace();

      match clock.next() {
        Some(clock) => match clock.parse::<i32>() {
          Ok(clock) => Some(clock),
          Err(_) => None,
        },
        None => None,
      }
    })
    .collect::<Vec<i32>>();

  match clock_speeds.into_iter().max() {
    Some(max_clock) => {
      info!("NVIDIA GPU {clock_type} max clock speed: {max_clock}");
      Ok(max_clock)
    }
    None => Err(anyhow!("Unable to get NVIDIA GPU {clock_type} max clock speed")),
  }
}


// Ref: https://developer.nvidia.com/blog/advanced-api-performance-setstablepowerstate/
pub fn set_nvidia_max_clock() -> Result<()> {
  info!("Setting NVIDIA GPU to max clock");

  // Enable persistence mode - enabled first since it makes
  // nvidia-smi commands execute faster when enabled
  cmd_exec("nvidia-smi", vec!["-pm", "1"])?;

  let graph_max_clock = get_nvidia_max_clock(&NvidiaGpuClock::Graphics)?;
  let mem_max_clock = get_nvidia_max_clock(&NvidiaGpuClock::Memory)?;

  // Disable autoboost since we are setting clocks to max
  cmd_exec("nvidia-smi", vec!["--auto-boost-default=0"])?;
  // Specifies <memory,graphics> clocks as a pair (e.g. 2000,800) in MHz
  cmd_exec(
    "nvidia-smi",
    vec!["--applications-clocks", &format!("{mem_max_clock},{graph_max_clock}")],
  )?;

  cmd_exec(
    "nvidia-smi",
    vec!["--lock-gpu-clocks", &format!("{mem_max_clock}")],
  )?;
  cmd_exec(
    "nvidia-smi",
    vec!["--lock-memory-clocks", &format!("{mem_max_clock}")],
  )?;

  Ok(())
}

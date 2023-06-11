use anyhow::Result;

/// Calculates the amount of memory to reserve for kubeReserved in mebibytes (Mi)
///
/// KubeReserved is a function of pod density so we are calculating the amount of
/// memory to reserve for Kubernetes systems daemons by considering the maximum
/// number of pods this instance type supports.
pub fn memory_mebibytes_to_reserve(max_pods: u32) -> Result<u32> {
  let reserve = 11 * max_pods + 255;
  Ok(reserve)
}

/// Calculates the amount of CPU to reserve for kubeReserved in millicores (mCPU) from the total number of vCPUs available on the instance
///
/// From the total core capacity of this worker node, we calculate the CPU resources to reserve by reserving a percentage
/// of the available cores in each range up to the total number of cores available on the instance.
/// 6% of the first core
/// 1% of the next core (up to 2 cores)
/// 0.5% of the next 2 cores (up to 4 cores)
/// 0.25% of any cores above 4 cores
/// 400mCPU is added when max pods > 110
pub fn cpu_millicores_to_reserve(max_pods: u32, num_cpus: u32) -> Result<u32> {
  let mut reserved = 0;
  for cpu in 0..num_cpus {
    match cpu {
      0 => reserved += 600,
      1 | 2 => reserved += 100,
      3 | 4 => reserved += 50,
      _ => reserved += 25,
    }
  }

  // 400mCPU is added when max pods > 110
  if max_pods > 110 {
    reserved += 400;
  }

  // Round to the nearest 10mCPU
  let reserved = ((reserved as f64 / 100.0).round() * 10.0) as u32;

  Ok(reserved)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cpu_millicores_to_reserve_t3nano() {
    let result = cpu_millicores_to_reserve(4, 2).unwrap();
    assert_eq!(result, 70);
  }

  #[test]
  fn memory_mebibytes_to_reserve_t3nano() {
    let result = memory_mebibytes_to_reserve(4).unwrap();
    assert_eq!(result, 299);
  }

  #[test]
  fn cpu_millicores_to_reserve_c524xlarge() {
    let result = cpu_millicores_to_reserve(250, 96).unwrap();
    assert_eq!(result, 360);
  }

  #[test]
  fn memory_mebibytes_to_reserve_c524xlarge() {
    let result = memory_mebibytes_to_reserve(250).unwrap();
    assert_eq!(result, 3005);
  }
}

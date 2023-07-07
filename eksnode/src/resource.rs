use anyhow::Result;

use crate::utils;

/// Calculates the amount of memory to reserve for kubeReserved in mebibytes (Mi)
///
/// KubeReserved is a function of pod density so we are calculating the amount of
/// memory to reserve for Kubernetes systems daemons by considering the maximum
/// number of pods this instance type supports.
pub fn memory_mebibytes_to_reserve(max_pods: i32) -> Result<i32> {
  let reserve = 11 * max_pods + 255;
  Ok(reserve)
}

/// Calculates the amount of CPU to reserve for kubeReserved in millicores (mCPU) from the total number of vCPUs
/// available on the instance
///
/// From the total core capacity of this worker node, we calculate the CPU resources to reserve by reserving a
/// percentage of the available cores in each range up to the total number of cores available on the instance.
/// 6% of the first core
/// 1% of the next core (up to 2 cores)
/// 0.5% of the next 2 cores (up to 4 cores)
/// 0.25% of any cores above 4 cores
/// 400mCPU is added when max pods > 110
pub fn cpu_millicores_to_reserve(max_pods: i32, num_cpus: i32) -> Result<i32> {
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
  let reserved = ((reserved as f64 / 100.0).round() * 10.0) as i32;

  Ok(reserved)
}

/// Calculate the max number of pods an instance can theoretically support based on ENIs
///
/// If prefix delegation is enabled, /28 CIDRs are allocated per IP available on the ENI:
/// num ENIs * ((num of IPv4s per ENI - 1) * 16) + 2
///
/// And without prefix delegation:
/// num ENIs * (num of IPv4s per ENI - 1) + 2
pub fn calculate_eni_max_pods(num_enis: i32, ipv4_addrs: i32, enable_prefix_del: bool) -> i32 {
  let modifier = if enable_prefix_del { 16 } else { 1 };

  num_enis * ((ipv4_addrs - 1) * modifier) + 2
}

/// Evaluate if the CNI version supports prefix delegation
///
/// https://docs.aws.amazon.com/eks/latest/userguide/cni-increase-ip-addresses.html
pub fn prefix_delegation_supported(cni_ver: &str) -> Result<bool> {
  let min_supported = utils::get_semver("1.9.0")?;
  let cni_ver = utils::get_semver(cni_ver)?;

  Ok(cni_ver >= min_supported)
}

#[cfg(test)]
mod tests {
  use rstest::*;

  use super::*;

  #[rstest]
  #[case(4, 299)]
  #[case(250, 3005)]
  fn memory_mebibytes_to_reserve_test(#[case] max_pods: i32, #[case] expected: i32) {
    let result = memory_mebibytes_to_reserve(max_pods).unwrap();
    assert_eq!(expected, result);
  }

  #[rstest]
  #[case(4, 2, 70)]
  #[case(250, 96, 360)]
  fn cpu_millicores_to_reserve_test(#[case] max_pods: i32, #[case] num_cpus: i32, #[case] expected: i32) {
    let result = cpu_millicores_to_reserve(max_pods, num_cpus).unwrap();
    assert_eq!(expected, result);
  }

  #[rstest]
  #[case(2, 4, false, 8)] // c6g.medium
  #[case(3, 10, false, 29)] // c5.large
  #[case(4, 15, false, 58)] // c5.xlarge/2xlarge
  #[case(8, 30, false, 234)] // c5.4xlarge/9xlarge/12xlarge
  #[case(7, 50, false, 345)] // c6in.32xlarge
  #[case(15, 50, false, 737)] // c5.18xlarge/24xlarge/metal
  #[case(4, 5, false, 18)] // d3.2xlarge
  #[case(4, 10, false, 38)] // d3.4xlarge
  #[case(3, 20, false, 59)] // d3.8xlarge
  #[case(4, 3, false, 10)] // d3.xlarge
  #[case(3, 30, false, 89)] // d3.12xlarge
  #[case(4, 20, false, 78)] // d3en.8xlarge
  #[case(8, 50, false, 394)] // f1.16xlarge
  #[case(4, 30, false, 118)] // g5g.4xlarge
  #[case(3, 15, false, 44)] // g5g.xlarge
  #[case(11, 30, false, 321)] // inf1.24xlarge
  #[case(5, 50, false, 247)] // trn1.32xlarge
  #[case(5, 30, false, 147)] // u-12tb1.metal
  #[case(2, 6, false, 12)] // m1.medium
  #[case(2, 10, false, 20)] // m4.large
  #[case(2, 2, false, 4)] // t1.micro
  #[case(3, 12, false, 35)] // t2.large
  #[case(3, 6, false, 17)] // t2.medium
  #[case(3, 4, false, 11)] // t2.small
  fn calculate_eni_max_pods_test(
    #[case] num_enis: i32,
    #[case] ipv4_addrs: i32,
    #[case] enable_prefix_del: bool,
    #[case] expected: i32,
  ) {
    let result = calculate_eni_max_pods(num_enis, ipv4_addrs, enable_prefix_del);
    assert_eq!(expected, result);
  }

  #[rstest]
  #[case("1.8.0", false)]
  #[case("1.9.0", true)]
  #[case("1.10.0", true)]
  #[case("v1.9.0-eksbuild.2", true)]
  #[case("v1.8.0-eksbuild.1", false)]
  #[case("v1.7.10-eksbuild.2", false)]
  #[case("v1.6.3-eksbuild.2", false)]
  #[should_panic]
  #[case("foo", false)]
  #[should_panic]
  #[case("", false)]
  #[should_panic]
  #[case(" ", false)]
  fn prefix_delegation_supported_test(#[case] cni_ver: &str, #[case] expected: bool) {
    let result = prefix_delegation_supported(cni_ver).unwrap();
    assert_eq!(expected, result);
  }
}

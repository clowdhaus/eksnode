use std::{
  collections::BTreeMap,
  fs::{File, OpenOptions},
  io::{BufReader, BufWriter},
  net::IpAddr,
  os::unix::fs::{chown, OpenOptionsExt},
  path::Path,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// KubeletConfiguration contains the configuration for the Kubelet
///
/// https://kubernetes.io/docs/tasks/administer-cluster/kubelet-config-file/
/// https://kubernetes.io/docs/reference/config-api/kubelet-config.v1beta1/
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KubeletConfiguration {
  /// Kind is a string value representing the REST resource this object represents.
  kind: String,

  /// APIVersion defines the versioned schema of this representation of an object.
  api_version: String,

  /// enableServer enables Kubelet's secured server.
  /// Note: Kubelet's insecure port is controlled by the readOnlyPort option.
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_server: Option<bool>,

  /// staticPodPath is the path to the directory containing local (static) pods to
  /// run, or the path to a single static pod file.
  #[serde(skip_serializing_if = "Option::is_none")]
  static_pod_path: Option<String>,

  /// syncFrequency is the max period between synchronizing running
  /// containers and config.
  #[serde(skip_serializing_if = "Option::is_none")]
  sync_frequency: Option<String>,

  /// fileCheckFrequency is the duration between checking config files for new data.
  #[serde(skip_serializing_if = "Option::is_none")]
  file_check_frequency: Option<String>,

  /// httpCheckFrequency is the duration between checking http for new data.
  #[serde(skip_serializing_if = "Option::is_none")]
  http_check_frequency: Option<String>,

  /// staticPodURL is the URL for accessing static pods to run.
  #[serde(rename = "staticPodURL", skip_serializing_if = "Option::is_none")]
  static_pod_url: Option<String>,

  /// staticPodURLHeader is a map of slices with HTTP headers to use when accessing the podURL.
  #[serde(rename = "staticPodURLHeader", skip_serializing_if = "Option::is_none")]
  static_pod_url_header: Option<BTreeMap<String, Vec<String>>>,

  /// address is the IP address for the Kubelet to serve on (set to 0.0.0.0
  /// for all interfaces).
  #[serde(skip_serializing_if = "Option::is_none")]
  address: Option<String>,

  /// port is the port for the Kubelet to serve on.
  /// The port number must be between 1 and 65535, inclusive.
  #[serde(skip_serializing_if = "Option::is_none")]
  port: Option<i32>,

  /// readOnlyPort is the read-only port for the Kubelet to serve on with
  /// no authentication/authorization.
  /// The port number must be between 1 and 65535, inclusive.
  /// Setting this field to 0 disables the read-only service.
  #[serde(skip_serializing_if = "Option::is_none")]
  read_only_port: Option<i32>,

  /// tlsCertFile is the file containing x509 Certificate for HTTPS. (CA cert,
  /// if any, concatenated after server cert). If tlsCertFile and
  /// tlsPrivateKeyFile are not provided, a self-signed certificate
  /// and key are generated for the public address and saved to the directory
  /// passed to the Kubelet's --cert-dir flag.
  #[serde(skip_serializing_if = "Option::is_none")]
  tls_cert_file: Option<String>,

  /// tlsPrivateKeyFile is the file containing x509 private key matching tlsCertFile.
  #[serde(skip_serializing_if = "Option::is_none")]
  tls_private_key_file: Option<String>,

  /// tlsCipherSuites is the list of allowed cipher suites for the server.
  /// Note that TLS 1.3 ciphersuites are not configurable.
  /// Values are from tls package constants (https://golang.org/pkg/crypto/tls/#pkg-constants).
  #[serde(skip_serializing_if = "Option::is_none")]
  tls_cipher_suites: Option<Vec<String>>,

  /// tlsMinVersion is the minimum TLS version supported.
  /// Values are from tls package constants (https://golang.org/pkg/crypto/tls/#pkg-constants).
  #[serde(skip_serializing_if = "Option::is_none")]
  tls_min_version: Option<String>,

  /// rotateCertificates enables client certificate rotation. The Kubelet will request a
  /// new certificate from the certificates.k8s.io API. This requires an approver to approve the
  /// certificate signing requests.
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_certificates: Option<bool>,

  /// serverTLSBootstrap enables server certificate bootstrap. Instead of self
  /// signing a serving certificate, the Kubelet will request a certificate from
  /// the 'certificates.k8s.io' API. This requires an approver to approve the
  /// certificate signing requests (CSR). The RotateKubeletServerCertificate feature
  /// must be enabled when setting this field.
  #[serde(rename = "serverTLSBootstrap", skip_serializing_if = "Option::is_none")]
  server_tls_bootstrap: Option<bool>,

  /// authentication specifies how requests to the Kubelet's server are authenticated.
  authentication: Authentication,

  /// authorization specifies how requests to the Kubelet's server are authorized.
  authorization: Authorization,

  /// registryPullQPS is the limit of registry pulls per second.
  /// The value must not be a negative number.
  /// Setting it to 0 means no limit.
  #[serde(rename = "registryPullQPS", skip_serializing_if = "Option::is_none")]
  registry_pull_qps: Option<i32>,

  /// registryBurst is the maximum size of bursty pulls, temporarily allows
  /// pulls to burst to this number, while still not exceeding registryPullQPS.
  /// The value must not be a negative number.
  /// Only used if registryPullQPS is greater than 0.
  #[serde(skip_serializing_if = "Option::is_none")]
  registry_burst: Option<i32>,

  /// eventRecordQPS is the maximum event creations per second. If 0, there
  /// is no limit enforced. The value cannot be a negative number.
  #[serde(rename = "eventRecordQPS", skip_serializing_if = "Option::is_none")]
  event_record_qps: Option<i32>,

  /// eventBurst is the maximum size of a burst of event creations, temporarily
  /// allows event creations to burst to this number, while still not exceeding
  /// eventRecordQPS. This field cannot be a negative number and it is only used
  /// when eventRecordQPS > 0.
  #[serde(skip_serializing_if = "Option::is_none")]
  event_burst: Option<i32>,

  /// enableDebuggingHandlers enables server endpoints for log access
  /// and local running of containers and commands, including the exec,
  /// attach, logs, and port forward features.
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_debugging_handlers: Option<bool>,

  /// enableContentionProfiling enables block profiling, if enableDebuggingHandlers is true.
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_contention_profiling: Option<bool>,

  /// healthzPort is the port of the localhost healthz endpoint (set to 0 to disable).
  /// A valid number is between 1 and 65535.
  #[serde(skip_serializing_if = "Option::is_none")]
  healthz_port: Option<i32>,

  /// healthzBindAddress is the IP address for the healthz server to serve on.
  #[serde(skip_serializing_if = "Option::is_none")]
  healthz_bind_address: Option<String>,

  /// oomScoreAdj is The oom-score-adj value for kubelet process. Values
  /// must be within the range [-1000, 1000].
  #[serde(skip_serializing_if = "Option::is_none")]
  oom_score_adj: Option<i32>,

  /// clusterDomain is the DNS domain for this cluster. If set, kubelet will
  /// configure all containers to search this domain in addition to the
  /// host's search domains.
  #[serde(skip_serializing_if = "Option::is_none")]
  cluster_domain: Option<String>,

  /// clusterDNS is a list of IP addresses for the cluster DNS server. If set,
  /// kubelet will configure all containers to use this for DNS resolution
  /// instead of the host's DNS servers.
  #[serde(rename = "clusterDNS", skip_serializing_if = "Option::is_none")]
  cluster_dns: Option<Vec<String>>,

  /// streamingConnectionIdleTimeout is the maximum time a streaming connection
  /// can be idle before the connection is automatically closed.
  #[serde(skip_serializing_if = "Option::is_none")]
  streaming_connection_idle_timeout: Option<String>,

  /// nodeStatusUpdateFrequency is the frequency that kubelet computes node
  /// status. If node lease feature is not enabled, it is also the frequency that
  /// kubelet posts node status to master.
  /// Note: When node lease feature is not enabled, be cautious when changing the
  /// constant, it must work with nodeMonitorGracePeriod in nodecontroller.
  #[serde(skip_serializing_if = "Option::is_none")]
  node_status_update_frequency: Option<String>,

  /// nodeStatusReportFrequency is the frequency that kubelet posts node
  /// status to master if node status does not change. Kubelet will ignore this
  /// frequency and post node status immediately if any change is detected. It is
  /// only used when node lease feature is enabled. nodeStatusReportFrequency's
  /// default value is 5m. But if nodeStatusUpdateFrequency is set explicitly,
  /// nodeStatusReportFrequency's default value will be set to
  /// nodeStatusUpdateFrequency for backward compatibility.
  #[serde(skip_serializing_if = "Option::is_none")]
  node_status_report_frequency: Option<String>,

  /// nodeLeaseDurationSeconds is the duration the Kubelet will set on its corresponding Lease.
  /// NodeLease provides an indicator of node health by having the Kubelet create and
  /// periodically renew a lease, named after the node, in the kube-node-lease namespace.
  /// If the lease expires, the node can be considered unhealthy.
  /// The lease is currently renewed every 10s, per KEP-0009. In the future, the lease renewal
  /// interval may be set based on the lease duration.
  /// The field value must be greater than 0.
  #[serde(skip_serializing_if = "Option::is_none")]
  node_lease_duration_seconds: Option<i32>,

  /// imageMinimumGCAge is the minimum age for an unused image before it is
  /// garbage collected.
  #[serde(rename = "imageMinimumGCAge", skip_serializing_if = "Option::is_none")]
  image_minimum_gc_age: Option<String>,

  /// imageGCHighThresholdPercent is the percent of disk usage after which
  /// image garbage collection is always run. The percent is calculated by
  /// dividing this field value by 100, so this field must be between 0 and
  /// 100, inclusive. When specified, the value must be greater than
  /// imageGCLowThresholdPercent.
  #[serde(rename = "imageGCHighThresholdPercent", skip_serializing_if = "Option::is_none")]
  image_gc_high_threshold_percent: Option<i32>,

  /// imageGCLowThresholdPercent is the percent of disk usage before which
  /// image garbage collection is never run. Lowest disk usage to garbage
  /// collect to. The percent is calculated by dividing this field value by 100,
  /// so the field value must be between 0 and 100, inclusive. When specified, the
  /// value must be less than imageGCHighThresholdPercent.
  #[serde(rename = "imageGCLowThresholdPercent", skip_serializing_if = "Option::is_none")]
  image_gc_low_threshold_percent: Option<i32>,

  /// volumeStatsAggPeriod is the frequency for calculating and caching volume
  /// disk usage for all pods.
  #[serde(skip_serializing_if = "Option::is_none")]
  volume_stats_agg_period: Option<String>,

  /// kubeletCgroups is the absolute name of cgroups to isolate the kubelet in
  #[serde(skip_serializing_if = "Option::is_none")]
  kubelet_cgroups: Option<String>,

  /// systemCgroups is absolute name of cgroups in which to place
  /// all non-kernel processes that are not already in a container. Empty
  /// for no container. Rolling back the flag requires a reboot.
  /// The cgroupRoot must be specified if this field is not empty.
  #[serde(skip_serializing_if = "Option::is_none")]
  cystem_cgroups: Option<String>,

  /// cgroupRoot is the root cgroup to use for pods. This is handled by the
  /// container runtime on a best effort basis.
  cgroup_root: Option<String>,

  /// cgroupsPerQOS enable QoS based CGroup hierarchy: top level CGroups for QoS classes
  /// and all Burstable and BestEffort Pods are brought up under their specific top level QoS CGroup.
  #[serde(rename = "cgroupsPerQOS", skip_serializing_if = "Option::is_none")]
  cgroups_per_qos: Option<String>,

  /// cgroupDriver is the driver kubelet uses to manipulate CGroups on the host (cgroupfs or systemd).
  #[serde(skip_serializing_if = "Option::is_none")]
  cgroup_driver: Option<String>,

  /// cpuManagerPolicy is the name of the policy to use.
  /// Requires the CPUManager feature gate to be enabled.
  #[serde(rename = "cpuManagerPolicy", skip_serializing_if = "Option::is_none")]
  cpu_manager_policy: Option<String>,

  /// cpuManagerPolicyOptions is a set of key=value which allows to set extra options
  /// to fine tune the behavior of the cpu manager policies.
  /// Requires  both the "CPUManager" and "CPUManagerPolicyOptions" feature gates to be enabled.
  #[serde(skip_serializing_if = "Option::is_none")]
  cpu_manager_policy_options: Option<BTreeMap<String, String>>,

  /// cpuManagerReconcilePeriod is the reconciliation period for the CPU Manager.
  /// Requires the CPUManager feature gate to be enabled.
  #[serde(skip_serializing_if = "Option::is_none")]
  cpu_manager_reconcile_period: Option<String>,

  /// memoryManagerPolicy is the name of the policy to use by memory manager.
  /// Requires the MemoryManager feature gate to be enabled.
  #[serde(skip_serializing_if = "Option::is_none")]
  memory_manager_policy: Option<String>,

  /// topologyManagerPolicy is the name of the topology manager policy to use.
  /// Valid values include:
  ///
  /// - `restricted`: kubelet only allows pods with optimal NUMA node alignment for requested resources;
  /// - `best-effort`: kubelet will favor pods with NUMA alignment of CPU and device resources;
  /// - `none`: kubelet has no knowledge of NUMA alignment of a pod's CPU and device resources.
  /// - `single-numa-node`: kubelet only allows pods with a single NUMA alignment of CPU and device resources.
  #[serde(skip_serializing_if = "Option::is_none")]
  topology_manager_policy: Option<String>,

  /// topologyManagerScope represents the scope of topology hint generation
  /// that topology manager requests and hint providers generate. Valid values include:
  ///
  /// - `container`: topology policy is applied on a per-container basis.
  /// - `pod`: topology policy is applied on a per-pod basis.
  #[serde(skip_serializing_if = "Option::is_none")]
  topology_manager_scope: Option<String>,

  /// TopologyManagerPolicyOptions is a set of key=value which allows to set extra options
  /// to fine tune the behavior of the topology manager policies.
  /// Requires  both the "TopologyManager" and "TopologyManagerPolicyOptions" feature gates to be enabled.
  #[serde(skip_serializing_if = "Option::is_none")]
  topology_manager_policy_options: Option<BTreeMap<String, String>>,

  /// qosReserved is a set of resource name to percentage pairs that specify
  /// the minimum percentage of a resource reserved for exclusive use by the
  /// guaranteed QoS tier.
  /// Currently supported resources: "memory"
  /// Requires the QOSReserved feature gate to be enabled.
  #[serde(rename = "qosReserved", skip_serializing_if = "Option::is_none")]
  qos_reserved: Option<BTreeMap<String, String>>,

  /// runtimeRequestTimeout is the timeout for all runtime requests except long running
  /// requests - pull, logs, exec and attach.
  #[serde(skip_serializing_if = "Option::is_none")]
  runtime_request_timeout: Option<String>,

  /// hairpinMode specifies how the Kubelet should configure the container
  /// bridge for hairpin packets.
  /// Setting this flag allows endpoints in a Service to load balance back to
  /// themselves if they should try to access their own Service. Values:
  ///
  /// - "promiscuous-bridge": make the container bridge promiscuous.
  /// - "hairpin-veth":       set the hairpin flag on container veth interfaces.
  /// - "none":               do nothing.
  ///
  /// Generally, one must set `--hairpin-mode=hairpin-veth to` achieve hairpin NAT,
  /// because promiscuous-bridge assumes the existence of a container bridge named cbr0.
  #[serde(skip_serializing_if = "Option::is_none")]
  hairpin_mode: Option<HairpinMode>,

  /// maxPods is the maximum number of Pods that can run on this Kubelet.
  /// The value must be a non-negative integer.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_pods: Option<i32>,

  /// podCIDR is the CIDR to use for pod IP addresses, only used in standalone mode.
  /// In cluster mode, this is obtained from the control plane.
  #[serde(rename = "podCIDR", skip_serializing_if = "Option::is_none")]
  pod_cidr: Option<String>,

  /// podPidsLimit is the maximum number of PIDs in any pod.
  #[serde(skip_serializing_if = "Option::is_none")]
  pod_pids_limit: Option<i32>,

  /// resolvConf is the resolver configuration file used as the basis
  /// for the container DNS resolution configuration.
  /// If set to the empty string, will override the default and effectively disable DNS lookups.
  #[serde(skip_serializing_if = "Option::is_none")]
  resolv_conf: Option<String>,

  /// runOnce causes the Kubelet to check the API server once for pods,
  /// run those in addition to the pods specified by static pod files, and exit.
  #[serde(skip_serializing_if = "Option::is_none")]
  run_once: Option<bool>,

  /// cpuCFSQuota enables CPU CFS quota enforcement for containers that
  /// specify CPU limits.
  #[serde(rename = "cpuCFSQuota", skip_serializing_if = "Option::is_none")]
  cpu_cfs_quota: Option<bool>,

  /// cpuCFSQuotaPeriod is the CPU CFS quota period value, `cpu.cfs_period_us`.
  /// The value must be between 1 ms and 1 second, inclusive.
  /// Requires the CustomCPUCFSQuotaPeriod feature gate to be enabled.
  #[serde(rename = "cpuCFSQuotaPeriod", skip_serializing_if = "Option::is_none")]
  cpu_cfs_quota_period: Option<String>,

  /// nodeStatusMaxImages caps the number of images reported in Node.status.images.
  /// The value must be greater than -2.
  /// Note: If -1 is specified, no cap will be applied. If 0 is specified, no image is returned.
  #[serde(skip_serializing_if = "Option::is_none")]
  node_status_max_images: Option<i32>,

  /// maxOpenFiles is Number of files that can be opened by Kubelet process.
  /// The value must be a non-negative number.
  #[serde(skip_serializing_if = "Option::is_none")]
  max_open_files: Option<i64>,

  /// contentType is contentType of requests sent to apiserver.
  #[serde(skip_serializing_if = "Option::is_none")]
  content_type: Option<String>,

  /// kubeAPIQPS is the QPS to use while talking with kubernetes apiserver.
  #[serde(rename = "kubeAPIQPS", skip_serializing_if = "Option::is_none")]
  pub kube_api_qps: Option<i32>,

  /// kubeAPIBurst is the burst to allow while talking with kubernetes API server.
  /// This field cannot be a negative number.
  #[serde(rename = "kubeAPIBurst", skip_serializing_if = "Option::is_none")]
  pub kube_api_burst: Option<i32>,

  /// serializeImagePulls when enabled, tells the Kubelet to pull images one
  /// at a time. We recommend *not* changing the default value on nodes that
  /// run docker daemon with version  < 1.9 or an Aufs storage backend.
  /// Issue #10959 has more details.
  #[serde(skip_serializing_if = "Option::is_none")]
  serialize_image_pulls: Option<bool>,

  /// MaxParallelImagePulls sets the maximum number of image pulls in parallel.
  /// This field cannot be set if SerializeImagePulls is true.
  /// Setting it to nil means no limit.
  #[serde(skip_serializing_if = "Option::is_none")]
  max_parallel_image_pulls: Option<i32>,

  /// evictionHard is a map of signal names to quantities that defines hard eviction
  /// thresholds. For example: `{"memory.available": "300Mi"}`.
  /// To explicitly disable, pass a 0% or 100% threshold on an arbitrary resource.
  #[serde(skip_serializing_if = "Option::is_none")]
  eviction_hard: Option<BTreeMap<String, String>>,

  /// evictionSoft is a map of signal names to quantities that defines soft eviction thresholds.
  /// For example: `{"memory.available": "300Mi"}`.
  #[serde(skip_serializing_if = "Option::is_none")]
  eviction_soft: Option<BTreeMap<String, String>>,

  /// evictionSoftGracePeriod is a map of signal names to quantities that defines grace
  /// periods for each soft eviction signal. For example: `{"memory.available": "30s"}`.
  #[serde(skip_serializing_if = "Option::is_none")]
  eviction_soft_grace_period: Option<BTreeMap<String, String>>,

  /// evictionPressureTransitionPeriod is the duration for which the kubelet has to wait
  /// before transitioning out of an eviction pressure condition.
  #[serde(skip_serializing_if = "Option::is_none")]
  eviction_pressure_transition_period: Option<String>,

  /// evictionMaxPodGracePeriod is the maximum allowed grace period (in seconds) to use
  /// when terminating pods in response to a soft eviction threshold being met. This value
  /// effectively caps the Pod's terminationGracePeriodSeconds value during soft evictions.
  /// Note: Due to issue #64530, the behavior has a bug where this value currently just
  /// overrides the grace period during soft eviction, which can increase the grace
  /// period from what is set on the Pod. This bug will be fixed in a future release.
  #[serde(skip_serializing_if = "Option::is_none")]
  eviction_max_pod_grace_period: Option<i32>,

  /// evictionMinimumReclaim is a map of signal names to quantities that defines minimum reclaims,
  /// which describe the minimum amount of a given resource the kubelet will reclaim when
  /// performing a pod eviction while that resource is under pressure.
  /// For example: `{"imagefs.available": "2Gi"}`.
  #[serde(skip_serializing_if = "Option::is_none")]
  eviction_minimum_reclaim: Option<BTreeMap<String, String>>,

  /// podsPerCore is the maximum number of pods per core. Cannot exceed maxPods.
  /// The value must be a non-negative integer.
  /// If 0, there is no limit on the number of Pods.
  #[serde(skip_serializing_if = "Option::is_none")]
  pods_per_core: Option<i32>,

  /// enableControllerAttachDetach enables the Attach/Detach controller to
  /// manage attachment/detachment of volumes scheduled to this node, and
  /// disables kubelet from executing any attach/detach operations.
  /// Note: attaching/detaching CSI volumes is not supported by the kubelet,
  /// so this option needs to be true for that use case.
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_controller_attach_detach: Option<bool>,

  /// protectKernelDefaults, if true, causes the Kubelet to error if kernel
  /// flags are not as it expects. Otherwise the Kubelet will attempt to modify
  /// kernel flags to match its expectation.
  #[serde(skip_serializing_if = "Option::is_none")]
  protect_kernel_defaults: Option<bool>,

  /// makeIPTablesUtilChains, if true, causes the Kubelet ensures a set of iptables rules
  /// are present on host.
  /// These rules will serve as utility rules for various components, e.g. kube-proxy.
  /// The rules will be created based on iptablesMasqueradeBit and iptablesDropBit.
  #[serde(rename = "makeIPTablesUtilChains", skip_serializing_if = "Option::is_none")]
  make_iptables_util_chains: Option<bool>,

  /// iptablesMasqueradeBit is the bit of the iptables fwmark space to mark for SNAT.
  /// Values must be within the range [0, 31]. Must be different from other mark bits.
  /// Warning: Please match the value of the corresponding parameter in kube-proxy.
  #[serde(skip_serializing_if = "Option::is_none")]
  iptables_masquerade_bit: Option<i32>,

  /// iptablesDropBit is the bit of the iptables fwmark space to mark for dropping packets.
  /// Values must be within the range [0, 31]. Must be different from other mark bits.
  #[serde(skip_serializing_if = "Option::is_none")]
  iptables_drop_bit: Option<i32>,

  /// featureGates is a map of feature names to bools that enable or disable experimental
  /// features. This field modifies piecemeal the built-in default values from
  /// "k8s.io/kubernetes/pkg/features/kube_features.go".
  #[serde(skip_serializing_if = "Option::is_none")]
  pub feature_gates: Option<BTreeMap<String, bool>>,

  /// failSwapOn tells the Kubelet to fail to start if swap is enabled on the node.
  #[serde(skip_serializing_if = "Option::is_none")]
  fail_swap_on: Option<bool>,

  /// memorySwap configures swap memory available to container workloads.
  #[serde(skip_serializing_if = "Option::is_none")]
  memory_swap: Option<MemorySwapConfiguration>,

  /// containerLogMaxSize is a quantity defining the maximum size of the container log
  /// file before it is rotated. For example: "5Mi" or "256Ki".
  #[serde(skip_serializing_if = "Option::is_none")]
  container_log_max_size: Option<String>,

  /// containerLogMaxFiles specifies the maximum number of container log files that can
  /// be present for a container.
  #[serde(skip_serializing_if = "Option::is_none")]
  container_log_max_files: Option<i32>,

  /// configMapAndSecretChangeDetectionStrategy is a mode in which ConfigMap and Secret
  /// managers are running.
  #[serde(skip_serializing_if = "Option::is_none")]
  config_map_and_secret_change_detection_strategy: Option<ResourceChangeDetectionStrategy>,

  /* the following fields are meant for Node Allocatable */
  /// systemReserved is a set of ResourceName=ResourceQuantity (e.g. cpu=200m,memory=150G)
  /// pairs that describe resources reserved for non-kubernetes components.
  /// Currently only cpu and memory are supported.
  /// See http://kubernetes.io/docs/user-guide/compute-resources for more detail.
  #[serde(skip_serializing_if = "Option::is_none")]
  system_reserved: Option<BTreeMap<String, String>>,

  /// kubeReserved is a set of ResourceName=ResourceQuantity (e.g. cpu=200m,memory=150G) pairs
  /// that describe resources reserved for kubernetes system components.
  /// Currently cpu, memory and local storage for root file system are supported.
  /// See https://kubernetes.io/docs/concepts/configuration/manage-resources-containers/
  /// for more details.
  #[serde(skip_serializing_if = "Option::is_none")]
  kube_reserved: Option<BTreeMap<String, String>>,

  /// The reservedSystemCPUs option specifies the CPU list reserved for the host
  /// level system threads and kubernetes related threads. This provide a "static"
  /// CPU list rather than the "dynamic" list by systemReserved and kubeReserved.
  /// This option does not support systemReservedCgroup or kubeReservedCgroup.
  #[serde(rename = "reservedSystemCPUs", skip_serializing_if = "Option::is_none")]
  reserved_system_cpus: Option<String>,

  /// showHiddenMetricsForVersion is the previous version for which you want to show
  /// hidden metrics.
  /// Only the previous minor version is meaningful, other values will not be allowed.
  /// The format is `<major>.<minor>`, e.g.: `1.16`.
  /// The purpose of this format is make sure you have the opportunity to notice
  /// if the next release hides additional metrics, rather than being surprised
  /// when they are permanently removed in the release after that.
  #[serde(skip_serializing_if = "Option::is_none")]
  show_hidden_metrics_for_version: Option<String>,

  /// systemReservedCgroup helps the kubelet identify absolute name of top level CGroup used
  /// to enforce `systemReserved` compute resource reservation for OS system daemons.
  /// Refer to [Node Allocatable](https://kubernetes.io/docs/tasks/administer-cluster/reserve-compute-resources/#node-allocatable)
  /// doc for more information.
  #[serde(skip_serializing_if = "Option::is_none")]
  system_reserved_cgroup: Option<String>,

  /// kubeReservedCgroup helps the kubelet identify absolute name of top level CGroup used
  /// to enforce `KubeReserved` compute resource reservation for Kubernetes node system daemons.
  /// Refer to [Node Allocatable](https://kubernetes.io/docs/tasks/administer-cluster/reserve-compute-resources/#node-allocatable)
  /// doc for more information.
  #[serde(skip_serializing_if = "Option::is_none")]
  kube_reserved_cgroup: Option<String>,

  /// This flag specifies the various Node Allocatable enforcements that Kubelet needs to perform.
  /// This flag accepts a list of options. Acceptable options are `none`, `pods`,
  /// `system-reserved` and `kube-reserved`.
  /// If `none` is specified, no other options may be specified.
  /// When `system-reserved` is in the list, systemReservedCgroup must be specified.
  /// When `kube-reserved` is in the list, kubeReservedCgroup must be specified.
  /// This field is supported only when `cgroupsPerQOS` is set to true.
  /// Refer to [Node Allocatable](https://kubernetes.io/docs/tasks/administer-cluster/reserve-compute-resources/#node-allocatable)
  /// for more information.
  #[serde(skip_serializing_if = "Option::is_none")]
  enforce_node_allocatable: Option<Vec<String>>,

  /// A comma separated whitelist of unsafe sysctls or sysctl patterns (ending in `*`).
  /// Unsafe sysctl groups are `kernel.shm*`, `kernel.msg*`, `kernel.sem`, `fs.mqueue.*`,
  /// and `net.*`. For example: "`kernel.msg*,net.ipv4.route.min_pmtu`"
  #[serde(skip_serializing_if = "Option::is_none")]
  allowed_unsafe_sysctls: Option<Vec<String>>,

  /// volumePluginDir is the full path of the directory in which to search
  /// for additional third party volume plugins.
  /// Default: "/usr/libexec/kubernetes/kubelet-plugins/volume/exec/"
  #[serde(skip_serializing_if = "Option::is_none")]
  volume_plugin_dir: Option<String>,

  /// providerID, if set, sets the unique ID of the instance that an external
  /// provider (i.e. cloudprovider) can use to identify a specific node.
  #[serde(rename = "providerID", skip_serializing_if = "Option::is_none")]
  pub provider_id: Option<String>,

  /// kernelMemcgNotification, if set, instructs the kubelet to integrate with the
  /// kernel memcg notification for determining if memory eviction thresholds are
  /// exceeded rather than polling.
  #[serde(skip_serializing_if = "Option::is_none")]
  kernel_memcg_notification: Option<bool>,

  /// logging specifies the options of logging.
  /// Refer to [Logs Options](https://github.com/kubernetes/component-base/blob/master/logs/options.go)
  /// for more information.
  #[serde(skip_serializing_if = "Option::is_none")]
  logging: Option<LoggingConfiguration>,

  /// enableSystemLogHandler enables system logs via web interface host:port/logs/
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_system_log_handler: Option<bool>,

  /// enableSystemLogQuery enables the node log query feature on the /logs endpoint.
  /// EnableSystemLogHandler has to be enabled in addition for this feature to work.
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_system_log_query: Option<bool>,

  /// shutdownGracePeriod specifies the total duration that the node should delay the
  /// shutdown and total grace period for pod termination during a node shutdown.
  #[serde(skip_serializing_if = "Option::is_none")]
  shutdown_grace_period: Option<String>,

  /// shutdownGracePeriodCriticalPods specifies the duration used to terminate critical
  /// pods during a node shutdown. This should be less than shutdownGracePeriod.
  /// For example, if shutdownGracePeriod=30s, and shutdownGracePeriodCriticalPods=10s,
  /// during a node shutdown the first 20 seconds would be reserved for gracefully
  /// terminating normal pods, and the last 10 seconds would be reserved for terminating
  /// critical pods.
  #[serde(skip_serializing_if = "Option::is_none")]
  shutdown_grace_period_critical_pods: Option<String>,

  /// shutdownGracePeriodByPodPriority specifies the shutdown grace period for Pods based
  /// on their associated priority class value.
  /// When a shutdown request is received, the Kubelet will initiate shutdown on all pods
  /// running on the node with a grace period that depends on the priority of the pod,
  /// and then wait for all pods to exit.
  /// Each entry in the array represents the graceful shutdown time a pod with a priority
  /// class value that lies in the range of that value and the next higher entry in the
  /// list when the node is shutting down.
  /// For example, to allow critical pods 10s to shutdown, priority>=10000 pods 20s to
  /// shutdown, and all remaining pods 30s to shutdown.
  ///
  /// shutdownGracePeriodByPodPriority:
  ///   - priority: 2000000000 shutdownGracePeriodSeconds: 10
  ///   - priority: 10000 shutdownGracePeriodSeconds: 20
  ///   - priority: 0 shutdownGracePeriodSeconds: 30
  ///
  /// The time the Kubelet will wait before exiting will at most be the maximum of all
  /// shutdownGracePeriodSeconds for each priority class range represented on the node.
  /// When all pods have exited or reached their grace periods, the Kubelet will release
  /// the shutdown inhibit lock.
  /// Requires the GracefulNodeShutdown feature gate to be enabled.
  /// This configuration must be empty if either ShutdownGracePeriod or ShutdownGracePeriodCriticalPods is set.
  #[serde(skip_serializing_if = "Option::is_none")]
  shutdown_grace_period_by_pod_priority: Option<Vec<ShutdownGracePeriodByPodPriority>>,

  /// reservedMemory specifies a comma-separated list of memory reservations for NUMA nodes.
  /// The parameter makes sense only in the context of the memory manager feature.
  /// The memory manager will not allocate reserved memory for container workloads.
  /// For example, if you have a NUMA0 with 10Gi of memory and the reservedMemory was
  /// specified to reserve 1Gi of memory at NUMA0, the memory manager will assume that
  /// only 9Gi is available for allocation.
  /// You can specify a different amount of NUMA node and memory types.
  /// You can omit this parameter at all, but you should be aware that the amount of
  /// reserved memory from all NUMA nodes should be equal to the amount of memory specified
  /// by the [node allocatable](https://kubernetes.io/docs/tasks/administer-cluster/reserve-compute-resources/#node-allocatable).
  /// If at least one node allocatable parameter has a non-zero value, you will need
  /// to specify at least one NUMA node.
  /// Also, avoid specifying:
  ///
  /// 1. Duplicates, the same NUMA node, and memory type, but with a different value.
  /// 2. zero limits for any memory type.
  /// 3. NUMAs nodes IDs that do not exist under the machine.
  /// 4. memory types except for memory and hugepages-<size>
  #[serde(skip_serializing_if = "Option::is_none")]
  reserved_memory: Option<Vec<MemoryReservation>>,

  /// enableProfilingHandler enables profiling via web interface host:port/debug/pprof/
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_profiling_handler: Option<bool>,

  /// enableDebugFlagsHandler enables flags endpoint via web interface host:port/debug/flags/v
  #[serde(skip_serializing_if = "Option::is_none")]
  enable_debug_flags_handler: Option<bool>,

  /// SeccompDefault enables the use of `RuntimeDefault` as the default seccomp profile for all workloads.
  #[serde(skip_serializing_if = "Option::is_none")]
  seccomp_default: Option<bool>,

  /// MemoryThrottlingFactor specifies the factor multiplied by the memory limit or node allocatable memory
  /// when setting the cgroupv2 memory.high value to enforce MemoryQoS.
  /// Decreasing this factor will set lower high limit for container cgroups and put heavier reclaim pressure
  /// while increasing will put less reclaim pressure.
  /// See https://kep.k8s.io/2570 for more details.
  #[serde(skip_serializing_if = "Option::is_none")]
  memory_throttling_factor: Option<f64>,

  /// registerWithTaints are an array of taints to add to a node object when
  /// the kubelet registers itself. This only takes effect when registerNode
  /// is true and upon the initial registration of the node.
  #[serde(skip_serializing_if = "Option::is_none")]
  register_with_taints: Option<Vec<Taint>>,

  /// registerNode enables automatic registration with the apiserver.
  #[serde(skip_serializing_if = "Option::is_none")]
  register_node: Option<bool>,

  /// Tracing specifies the versioned configuration for OpenTelemetry tracing clients.
  /// See https://kep.k8s.io/2832 for more details.
  #[serde(skip_serializing_if = "Option::is_none")]
  tracing: Option<TracingConfiguration>,

  /// LocalStorageCapacityIsolation enables local ephemeral storage isolation feature. The default setting is true.
  /// This feature allows users to set request/limit for container's ephemeral storage and manage it in a similar way
  /// as cpu and memory. It also allows setting sizeLimit for emptyDir volume, which will trigger pod eviction if disk
  /// usage from the volume exceeds the limit.
  /// This feature depends on the capability of detecting correct root file system disk usage. For certain systems,
  /// such as kind rootless, if this capability cannot be supported, the feature LocalStorageCapacityIsolation should
  /// be disabled. Once disabled, user should not set request/limit for container's ephemeral storage, or sizeLimit
  /// for emptyDir.
  #[serde(skip_serializing_if = "Option::is_none")]
  local_storage_capacity_isolation: Option<bool>,

  /// ContainerRuntimeEndpoint is the endpoint of container runtime.
  /// Unix Domain Sockets are supported on Linux, while npipe and tcp endpoints are supported on Windows.
  /// Examples:'unix:///path/to/runtime.sock', 'npipe:////./pipe/runtime'
  #[serde(skip_serializing_if = "Option::is_none")]
  container_runtime_endpoint: Option<String>,

  /// ImageServiceEndpoint is the endpoint of container image service.
  /// Unix Domain Socket are supported on Linux, while npipe and tcp endpoints are supported on Windows.
  /// Examples:'unix:///path/to/runtime.sock', 'npipe:////./pipe/runtime'.
  /// If not specified, the value in containerRuntimeEndpoint is used.
  #[serde(skip_serializing_if = "Option::is_none")]
  image_service_endpoint: Option<String>,
}

impl KubeletConfiguration {
  pub fn new(cluster_dns: IpAddr, mebibytes_to_reserve: i32, cpu_millicores_to_reserve: i32) -> Self {
    KubeletConfiguration {
      kind: "KubeletConfiguration".to_string(),
      api_version: "kubelet.config.k8s.io/v1beta1".to_string(),
      address: Some("0.0.0.0".to_string()),
      authentication: Authentication {
        anonymous: AuthnAnonymous { enabled: false },
        webhook: AuthnWebhook {
          cache_ttl: "2m0s".to_string(),
          enabled: true,
        },
        x509: AuthnX509 {
          client_ca_file: "/etc/kubernetes/pki/ca.crt".to_string(),
        },
      },
      authorization: Authorization {
        mode: "Webhook".to_string(),
        webhook: AuthzWebhook {
          cache_authorized_ttl: "5m0s".to_string(),
          cache_unauthorized_ttl: "30s".to_string(),
        },
      },
      cluster_domain: Some("cluster.local".to_string()),
      cluster_dns: Some(vec![cluster_dns.to_string()]),
      container_runtime_endpoint: Some("unix:///run/containerd/containerd.sock".to_string()),
      eviction_hard: Some(BTreeMap::from([
        ("memory.available".to_string(), "100Mi".to_string()),
        ("nodefs.available".to_string(), "10%".to_string()),
        ("nodefs.inodesFree".to_string(), "5%".to_string()),
      ])),
      kube_reserved: Some(BTreeMap::from([
        ("cpu".to_string(), format!("{cpu_millicores_to_reserve}m")),
        ("ephemeral-storage".to_string(), "3Gi".to_string()),
        ("memory".to_string(), format!("{mebibytes_to_reserve}Mi")),
      ])),
      hairpin_mode: Some(HairpinMode::HairpinVeth),
      read_only_port: Some(0),
      cgroup_driver: Some("systemd".to_string()),
      cgroup_root: Some("/".to_string()),
      system_reserved_cgroup: Some("/system".to_string()),
      kube_reserved_cgroup: Some("/runtime".to_string()),
      feature_gates: Some(BTreeMap::from([("RotateKubeletServerCertificate".to_string(), true)])),
      protect_kernel_defaults: Some(true),
      serialize_image_pulls: Some(false),
      server_tls_bootstrap: Some(true),
      shutdown_grace_period: Some("45s".to_string()),
      shutdown_grace_period_critical_pods: Some("15s".to_string()),
      tls_cipher_suites: Some(vec![
        "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256".to_string(),
        "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256".to_string(),
        "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305".to_string(),
        "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string(),
        "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305".to_string(),
        "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384".to_string(),
        "TLS_RSA_WITH_AES_256_GCM_SHA384".to_string(),
        "TLS_RSA_WITH_AES_128_GCM_SHA256".to_string(),
      ]),
      ..KubeletConfiguration::default()
    }
  }

  /// The unique ID of the instance that an external provider (i.e. cloudprovider) can use to identify a specific node
  ///
  /// Only used when the cloud provider is external (< 1.27)
  pub fn get_provider_id(&self, availability_zone: &str, instance_id: &str) -> Result<String> {
    Ok(format!("aws:///{availability_zone}/{instance_id}"))
  }

  pub fn read<P: AsRef<Path>>(path: P) -> Result<Self> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let conf: KubeletConfiguration = serde_json::from_reader(reader)?;

    Ok(conf)
  }

  pub fn write<P: AsRef<Path>>(&self, path: P, id: Option<u32>) -> Result<()> {
    let file = OpenOptions::new().write(true).create(true).mode(0o644).open(&path)?;
    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(writer, self).map_err(anyhow::Error::from)?;
    Ok(chown(&path, id, id)?)
  }
}

/// HairpinMode denotes how the kubelet should configure networking
/// to handle hairpin packets
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HairpinMode {
  /// Set the hairpin flag on the veth of containers in the respective
  /// container runtime.
  HairpinVeth,
  /// Make the container bridge promiscuous. This will force it to accept
  /// hairpin packets, even if the flag isn't set on ports of the bridge.
  PromiscuousBridge,
  /// Neither of the above. If the kubelet is started in this hairpin mode
  /// and kube-proxy is running in iptables mode, hairpin packets will be
  /// dropped by the container bridge.
  HairpinNone,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Authentication {
  anonymous: AuthnAnonymous,
  webhook: AuthnWebhook,
  x509: AuthnX509,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename = "anonymous")]
pub struct AuthnAnonymous {
  enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename = "webhook")]
pub struct AuthnWebhook {
  #[serde(rename = "cacheTTL")]
  cache_ttl: String,
  enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename = "x509")]
pub struct AuthnX509 {
  #[serde(rename = "clientCAFile")]
  client_ca_file: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorization {
  mode: String,
  webhook: AuthzWebhook,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename = "webhook")]
pub struct AuthzWebhook {
  #[serde(rename = "cacheAuthorizedTTL")]
  cache_authorized_ttl: String,
  #[serde(rename = "cacheUnauthorizedTTL")]
  cache_unauthorized_ttl: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySwapConfiguration {
  /// swapBehavior configures swap memory available to container workloads. May be one of
  /// "", "LimitedSwap": workload combined memory and swap usage cannot exceed pod memory limit
  /// "UnlimitedSwap": workloads can use unlimited swap, up to the allocatable limit.
  swap_behavior: Option<String>,
}

/// Denotes a mode in which internal managers (secret, configmap) are discovering object changes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ResourceChangeDetectionStrategy {
  /// kubelet fetches necessary objects directly from the API server
  Get,
  /// kubelet uses TTL cache for object fetched from the API server
  Cache,
  /// kubelet uses watches to observe changes to objects that are in its interest
  Watch,
}

// Specifies the shutdown grace period for Pods based on their associated priority class value
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownGracePeriodByPodPriority {
  /// priority is the priority value associated with the shutdown grace period
  priority: i32,

  /// shutdownGracePeriodSeconds is the shutdown grace period in seconds
  shutdown_grace_period_seconds: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taint {
  /// Required. The taint key to be applied to a node.
  key: String,
  /// Required. The taint value corresponding to the taint key.
  /// +optional
  value: String,
  /// Required. The effect of the taint on pods
  /// that do not tolerate the taint.
  /// Valid effects are NoSchedule, PreferNoSchedule and NoExecute.
  effect: String,
  /// TimeAdded represents the time at which the taint was added.
  /// It is only written for NoExecute taints.
  time_added: String,
}

// MemoryReservation specifies the memory reservation of different types for each NUMA node
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReservation {
  numa_node: i32,
  limits: BTreeMap<String, String>,
}

/// LoggingConfiguration contains logging options
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfiguration {
  /// Format Flag specifies the structure of log messages.
  format: Option<String>,

  /// Maximum number of nanoseconds (i.e. 1s = 1000000000) between log
  /// flushes. Ignored if the selected logging backend writes log
  /// messages without buffering.
  flush_requency: String,

  /// Verbosity is the threshold that determines which log messages are
  /// logged. Default is zero which logs only the most important messages.
  /// Higher values enable additional messages. Error messages are always logged.
  verbosity: u32,

  /// VModule overrides the verbosity threshold for individual files.
  /// Only supported for "text" log format.
  #[serde(skip_serializing_if = "Option::is_none")]
  vmodule: Option<Vec<VModuleItem>>,

  /// [Alpha] Options holds additional parameters that are specific
  /// to the different logging formats. Only the options for the selected
  /// format get used, but all of them get validated.
  /// Only available when the LoggingAlphaOptions feature gate is enabled.
  #[serde(flatten, skip_serializing_if = "Option::is_none")]
  options: Option<BTreeMap<String, serde_json::Value>>,
}

// TracingConfiguration provides versioned configuration for OpenTelemetry tracing clients.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracingConfiguration {
  /// Endpoint of the collector this component will report traces to.
  /// The connection is insecure, and does not currently support TLS.
  /// Recommended is unset, and endpoint is the otlp grpc default, localhost:4317.
  endpoint: Option<String>,

  /// SamplingRatePerMillion is the number of samples to collect per million spans.
  /// Recommended is unset. If unset, sampler respects its parent span's sampling
  /// rate, but otherwise never samples.
  sampling_rate_per_million: Option<i32>,
}

/// Defines verbosity for one or more files which match a certain glob pattern
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VModuleItem {
  /// FilePattern is a base file name (i.e. minus the ".go" suffix and
  /// directory) or a "glob" pattern for such a name. It must not contain
  /// comma and equal signs because those are separators for the
  /// corresponding klog command line argument.
  file_pattern: String,

  /// Verbosity is the threshold for log messages emitted inside files
  /// that match the pattern.
  verbosity: u32,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_serializes_kubelet_config() {
    let config = r#"{
      "kind": "KubeletConfiguration",
      "apiVersion": "kubelet.config.k8s.io/v1",
      "address": "0.0.0.0",
      "authentication": {
        "anonymous": {
          "enabled": false
        },
        "webhook": {
          "cacheTTL": "2m0s",
          "enabled": true
        },
        "x509": {
          "clientCAFile": "/etc/kubernetes/pki/ca.crt"
        }
      },
      "authorization": {
        "mode": "Webhook",
        "webhook": {
          "cacheAuthorizedTTL": "5m0s",
          "cacheUnauthorizedTTL": "30s"
        }
      },
      "clusterDomain": "cluster.local",
      "hairpinMode": "hairpin-veth",
      "readOnlyPort": 0,
      "cgroupDriver": "cgroupfs",
      "cgroupRoot": "/",
      "featureGates": {
        "RotateKubeletServerCertificate": true,
        "KubeletCredentialProviders": true
      },
      "protectKernelDefaults": true,
      "serializeImagePulls": false,
      "serverTLSBootstrap": true,
      "tlsCipherSuites": [
        "TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256",
        "TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256",
        "TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305",
        "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384",
        "TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305",
        "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384",
        "TLS_RSA_WITH_AES_256_GCM_SHA384",
        "TLS_RSA_WITH_AES_128_GCM_SHA256"
      ]
    }"#;

    let deserialized: KubeletConfiguration = serde_json::from_str(config).unwrap();
    insta::assert_debug_snapshot!(deserialized);

    let serialized = serde_json::to_string(&deserialized).unwrap();
    insta::assert_debug_snapshot!(serialized);
  }
}

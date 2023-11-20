
## Image vs Bootstrap

There are primarily two phases - image creation and node provisioning. The image creation process is responsible for installing components and setting static configuration while the node provisioning process is responsible for performing dynamic configuration that cannot, or should not, be baked into the image. There is a tradeoff between image creation and node provisioning with respect to image flexibility and size versus the time required to join the cluster.

The more that can be baked into the image, the less time that is required for the node to join the cluster. However, this also means that the image will be either less flexible with respect to the various configurations it supports or the image will increase in size resulting in slower startup times. Conversely, making the image more flexible by moving more tasks to the node provisioning process will result in more time required for the node to join the cluster.

### Image

As much work as possible should be performed during the image creation process in order to minimize the amount of time and process activity during node provisioning. The time penalty here is inconsequential when compared with the time penalty that would be incurred during node provisioning.

### Bootstrap

- kubelet kubeconfig needs to be updated with endpoint, cluster name, region, etc.
- Store cluster b64 decoded CA cert in `/etc/kubernetes/pki/ca.crt`
- Outpost configuration updates to `/etc/hosts` and kubelet kubeconfig
- Set the ECR repository based on region for pause container
- Kubelet args passed to systemd unit file
  - Move as many kubelet args to the kubelet config file as possible
    - <https://kubernetes.io/docs/tasks/administer-cluster/kubelet-config-file/>
    - <https://kubernetes.io/docs/reference/config-api/kubelet-config.v1beta1/>
- Mount/partition instance store volumes
- Add node labels
  - NFD & GFD common labels

#### Order of Operations

1. Collect instance data - try IMDS, fallback to describe API call
2. Get kubelet version (used for conditional logic across versions)
3. Write CA cert to `/etc/kubernetes/pki/ca.crt`
4. Write kubelet config to file (location depends on Outposts)
5. If on Outpost, update `/etc/hosts`
6. Get or calculate max pods for instance
7. Update containerd config - pause container + any user configuration overrides; reload/restart containerd
8. Create kubelet args and extra args systemd unit files; reload/restart kubelet

### Tenants

1. The image filesystem is the source of truth.

    Users can modify the filesystem via image creation or user data which means the CLI cannot be the source of truth. However, the CLI can aid in modifying attributes via user data.

### Terms

- Node provisioning: the process of turning on an EC2 instance, performing initialization/setup processes, and joining the node to the cluster

## Motivation

1. Fresh slate to build using the latest technologies

  |  | EKS AL2 AMI | eksnode |
  |: --- :|: --- :|: --- :|
  | Kernel | 5.10 | 6.1 |
  | Docker | 20.10 | ❌ |
  | containerd | 1.6 | 1.7 |
  | cgroup | v1 | v2 |

    - Add <https://github.com/awslabs/soci-snapshotter> by default
    - Add support for "profiles" for different workload types
      - Change default max PIDs
    - Add support for network proxy <https://github.com/awslabs/amazon-eks-ami/issues/1182>

2. Improve the development process and increase confidence in changes
    - Bash scripts + various *nix tools -> single, custom executable
      - Reduce the amount of additional tools required to facilitate joining the node to the cluster
    - Introduce unit tests, snapshot tests, and various integration tests
    - CLI provides ability to run various commands both locally and on host all through same interface/executable
      - Bootstrap, calc max pods, collect logs, validate configs, etc.
    - Improve troubleshooting through the use of debug logging controlled by variable flag

3. Provide a stable interface and better support for customization to override default values
    - Improved validation of input parameters
    - Support for customizing the AMI using a configuration file
    - Support for specifying core component versions
      - containerd, runc, CNI
      - NVIDIA driver, EFA, etc.
    - Support for providing a kubelet config file
      - Either replace or merge with default kubelet config
      - <https://github.com/awslabs/amazon-eks-ami/issues/661>

4. The ability to generate multiple AMI types/configurations
    - CIS compliant AMIs (default)
    - STIG, FedRAMP, FIPS compliant AMIs
      - <https://github.com/awslabs/amazon-eks-ami/pull/1028>
    - AMIs w/ GPU support (drivers and devices)

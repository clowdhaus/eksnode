## Motivation

1. Fresh slate to build using the latest technologies
    - AL2  -> AL2023
    - cgroup v1 -> v2
    - Docker is removed and containerd is the default
      - Option to add additional volume for containerd only
    - Add <https://github.com/awslabs/soci-snapshotter> by default
    - Add support for "profiles" for different workload types
      - Change default max PIDs
    - Add support for network proxy <https://github.com/awslabs/amazon-eks-ami/issues/1182>

2. Improve the development process and increase confidence in changes
    - Bash scripts + various *nix tools -> single, custom executable
      - Reduce the amount of additional tools that need to be installed to facilitate bootstrap process
    - Introduce unit tests, snapshot tests, and various integration tests
    - CLI provides ability to run various commands both locally and on host all through same interface/executable
      - Bootstrap, calc max pods, collect logs, validate configs, etc.
    - Improve troubleshooting through the use debug logging controlled by variable flag

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

## Image vs Bootstrap

There are tradeoffs when it comes to determining if a change should be made to the image or the node join process. Baking the changes into the image results in faster instance startup times but reduces the configuration flexibility and increase the amount of image variants. Making the changes during node provisioning results in longer startup times but allows for a more flexible and dynamic configuration. See below for more details on how we approached this tradeoff below and some of the factors that influenced our decisions.

### Image

As much work as possible should be performed during the image creation process in order to minimize the amount of time and process activity during node provisioning. The time penalty here is inconsequential when compared with the time penalty that would be incurred during node provisioning.

- Create `/etc/kubernetes/pki` directory
- ECR credential provider config - version dependent, not something users modify
- Create `/etc/containerd` and `/etc/cni/net.d` directories
  - Create systemd unit files and slices
- Create `/etc/systemd/system/kubelet.service.d` directory
- Mount BPF filesystem by default

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
- Containerd and kubelet path location
- Enable EFA interfaces
  - Configuration provided will allow customization of the EFA(s)
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

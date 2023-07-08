# eksnode

## Motivation

1. Fresh slate to build using the latest technologies
    - AL2 AMI -> AL2023
    - Cgroupsv1 -> Cgroupsv2
    - Docker is removed and only containerd is considered
      - Additional volume for containerd only
    - Add https://github.com/awslabs/soci-snapshotter by default
    - Change default max PIDs (increase)
    - Add support for network proxy https://github.com/awslabs/amazon-eks-ami/issues/1182
    - Add crictl by default

2. Improve the development process and increase confidence in changes
    - Bash scripts -> Rust binary
    - Introduce unit tests, snapshot tests, and various integration tests
    - CLI provides ability to run various commands both locally and on host all through same interface/executable
      - Bootstrap, calc max pods, collect logs, validate configs, etc.
    - Improve troubleshooting through the use debug logging controlled by variable flag

3. Provide a stable interface and better support for customization to override default values
    - Improved validation of input parameters
    - Support for customizing the AMI using a configuration file
    - Support for specifying core component versions
      - containerd & runc & CNI
      - NVIDIA drivers & CUDA
    - Support for providing a kubelet config file
      - Either replace or merge with default kubelet config
      - https://github.com/awslabs/amazon-eks-ami/issues/661

4. The ability to generate multiple AMI types/configurations
    - CIS compliant AMIs (default)
    - STIG, FedRAMP, FIPS compliant AMIs
      - https://github.com/awslabs/amazon-eks-ami/pull/1028
    - AMIs w/ GPU support (drivers and devices)

## Image vs Bootstrap

There are tradeoffs when it comes to determining if a change should be made to the image or the node join process. Baking the changes into the image results in faster instance startup times but reduces the configuration flexibility and increase the amount of image variants. Making the changes during node provisioning results in longer startup times but allows for a more flexible and dynamic configuration. See below for more details on how we approached this tradeoff below and some of the factors that influenced our decisions.

### Image

- Files located under `files/` directory are baked into the image
- Create `/etc/kubernetes/pki` directory
- ECR credential provider config - version dependent, not something users modify
- Create `/etc/containerd` and `/etc/cni/net.d` directories
  - Create systemd unit files and slices
- Create `/etc/systemd/system/kubelet.service.d` directory
  - Create systemd unit file for args and extra args
- Mount BPF filesystem by default

### Bootstrap

- kubelet kubeconfig needs to be updated with endpoint, cluster name, region, etc.
- Store cluster b64 decoded CA cert in `/etc/kubernetes/pki/ca.crt`
- Outpost configuration updates to `/etc/hosts` and kubelet kubeconfig
- Set the ECR repository based on region for pause container
- Kubelet args passed to systemd unit file
  - Move as many kubelet args to the kubelet config file as possible
    - https://kubernetes.io/docs/tasks/administer-cluster/kubelet-config-file/
    - https://kubernetes.io/docs/reference/config-api/kubelet-config.v1beta1/
- Instance store volume mount and configuration
- Containerd and kubelet mount location

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

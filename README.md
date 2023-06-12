# Experimental Amazon EKS AMI

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

There are tradeoffs when it comes to determining if a change should be made to the image or the bootstrap process. Baking the changes into the image results in faster instance startup times but reduces the configuration flexibility and increase the amount of image variants. Making the changes during instance bootstrapping results in longer instance startup times but allows for a more flexible and dynamic configuration. See below for more details on how we approached this tradeoff below and some of the factors that influenced our decisions.

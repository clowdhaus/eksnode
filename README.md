# Experimental Amazon EKS AMI

## Motivation

1. Fresh slate to build using the latest technologies
  - AL2 AMI -> AL2023
  - Cgroupsv1 -> Cgroupsv2
  - Docker is removed and only containerd is considered
  - Add https://github.com/awslabs/soci-snapshotter by default

2. Improve the development process and increase confidence in changes
  - Bash scripts -> Rust binary
  - Introduce unit tests, snapshot tests, and various integration tests
  - CLI provides ability to run various commands both locally and on host all through same interface/executable
    - Bootstrap, calc max pods, collect logs, validate configs, etc.

3. Provide a stable interface and better support for customization to override default values
  - Improved validation of input parameters
  - Support for customizing the AMI using a configuration file
  - Allow users to specify which images they would like to have cached in the AMI

4. The ability to generate multiple AMI types/configurations
  - CIS compliant AMIs (default)
  - STIG and FedRAMP compliant AMIs
  - AMIs w/ GPU support (drivers and devices)

5. Support for generating SBOMs

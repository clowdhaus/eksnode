# TODO

- [ ] Add Ansible role for NVIDIA GPU drivers and supporting software
- [ ] Add log collection functionality
  - Expand on this - this should be created from the perspective of `eksnode debug` perspective where `--collect-logs` can be used to collect logs from the node similar to current AMI functionality
- [ ] Add validate functionality
  - Will need to write a config file to use for validation - will capture the build-spec and join-spec so that when running `eksnode validate`, the default behavior will be to load this file to infer how the node was intended to be setup. Users should have the option to override and check specifics with something like `eksnode validate --nvidia-gpu` or `eksnode validate --containerd` or even `eksnode validate --containerd --nvidia-gpu`, etc.
- [ ] Add Ansible role for STIG hardening
- [ ] Add Ansible role for CIS hardening
- [ ] Add Ansible role for FedRAMP hardening
- [ ] Add checks and reporting of CIS, STIG, FedRAMP hardening with OpenSCAP
- [ ] Add SBOM generation capability

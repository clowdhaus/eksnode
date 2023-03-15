# Experimental Amazon EKS AMI

## Goals

1. Create AL2023 based EKS AMIs
2. Ability to create EKS AMIs that are hardened against OpenSCAP baseline(s)/profile(s)
3. Ability to routinely create and validate EKS AMIs using the Kubernetes conformance tests

### MVP 1

- [ ] Create AL2023 x86_64 & ARM64 EKS AMIs
- [ ] Validate AMIs on simple test EKS cluster (simple checks - connects to control plane, can run daemonset, etc.)

### MVP 2

- [ ] Able to scan EKS AMIs with OpenSCAP for CIS and STIG compliance
  - [ ] Ansible playbook to run OpenSCAP scan
  - [ ] Ansible playbook to harden
  - [ ] Packer Ansible remote provisioner

### MVP 3

- [ ] Automation
  - [ ] Build hardened AMI with OpenSCAP results
  - [ ] Validate hardened AMI with Kubernetes conformance tests
  - [ ] Generate SBOM for AMI

### Misc

- [ ] Use cgroups v2
- [ ] Set max PIDs

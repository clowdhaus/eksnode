# TODO

- [ ] Ability to enable/disable log colored output (plain text for cloud-init)
- [ ] Add Ansible role for NVIDIA GPU drivers and supporting software
- [ ] Add log collection functionality
  - Expand on this - this should be created from the perspective of `eksnode debug` perspective where `--collect-logs` can be used to collect logs from the node similar to current AMI functionality
- [ ] Add validate functionality
  - Will need to write a config file to use for validation - will capture the build-spec and join-spec so that when running `eksnode validate`, the default behavior will be to load this file to infer how the node was intended to be setup. Users should have the option to override and check specifics with something like `eksnode validate --nvidia-gpu` or `eksnode validate --containerd` or even `eksnode validate --containerd --nvidia-gpu`, etc.
- [ ] Add Ansible role for STIG hardening
- [ ] Add Ansible role for CIS hardening
- [ ] Add Ansible role for FedRAMP hardening
- [ ] Add checks and reporting of CIS, STIG, FedRAMP hardening with OpenSCAP
- [ ] Add functionality to generate SBOM
  - [ ] Both locally and remotely in S3 for use in AMI pipeline

## Max Pods

- [ ] Modify the max-pods calculation
  - One pod on a large instance should not carve out enough mem/cpu as if that instance will host the `max-pods` when its only intended to host a single pod (Spark workloads)
  - When utilizing GPUs/accelerators, `max-pods` is a combination of the number of GPUs attached to the instance and any concurrency configuration (time-slicing, MiG, etc.)
  - What algorithm is appropriate for allowing users to maximize the number of pods on an instance (i.e - can they schedule 737 pods on an instance safely and without resource contention?)
  - What tests can be used or constructed to test/validate these changes?
  - Karpenter
    - [Provisioners: Pod Density](https://karpenter.sh/preview/concepts/provisioners/#pod-density)
    - [Support for density optimized memory overhead](https://github.com/aws/karpenter/issues/1295)

## Validate Output

```sh
[ec2-user@ip-10-0-48-62 ~]$ eksnode validate
ERROR eksnode::commands::validate: /etc/eks/sandbox-image.service: No such file or directory (os error 2)
ERROR eksnode::commands::validate: /etc/eks/image-credential-provider/config.json: No such file or directory (os error 2)
ERROR eksnode::commands::validate: /etc/systemd/system/kubelet.service: No such file or directory (os error 2)
```

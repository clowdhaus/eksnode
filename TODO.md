# TODO

- [ ] Pre-fetch images on AMI <https://github.com/awslabs/amazon-eks-ami/pull/938>
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

## Containerd/Kubelet

- [ ] Merge default containerd config on host with config provided by users, if one is provided
- [ ] [Drop-in directory for kubelet configuration files](https://kubernetes.io/docs/tasks/administer-cluster/kubelet-config-file/#kubelet-conf-d)
- [ ] Add support for running containerd snapshotter benchmarks <https://github.com/containerd/containerd/tree/main/contrib>
  - Default snapshotter is overlayfs, are there any benefits with aufs/btrfs? What about when storage is located on instance store volume? Do we need to do anything additional for xfs when overlayfs is used?

## Max Pods

- [ ] Modify the max-pods calculation
  - One pod on a large instance should not carve out enough mem/cpu as if that instance will host the `max-pods` when its only intended to host a single pod (Spark workloads)
  - When utilizing GPUs/accelerators, `max-pods` is a combination of the number of GPUs attached to the instance and any concurrency configuration (time-slicing, MiG, etc.)
  - What algorithm is appropriate for allowing users to maximize the number of pods on an instance (i.e - can they schedule 737 pods on an instance safely and without resource contention?)
  - What tests can be used or constructed to test/validate these changes?
  - Karpenter
    - [Provisioners: Pod Density](https://karpenter.sh/preview/concepts/provisioners/#pod-density)
    - [Support for density optimized memory overhead](https://github.com/aws/karpenter/issues/1295)

## Wasi

- [ ] [WebAssembly and containerd: How it works](https://nigelpoulton.com/webassembly-and-containerd-how-it-works/)
  - [WebAssembly on Kubernetes: The ultimate hands-on guide](https://nigelpoulton.com/webassembly-on-kubernetes-ultimate-hands-on/)
  - [WebAssembly on Kubernetes: everything you need to know](https://nigelpoulton.com/webassembly-on-kubernetes-everything-you-need-to-know/)

## Validate Output

```sh
[ec2-user@ip-10-0-48-62 ~]$ eksnode validate
ERROR eksnode::commands::validate: /etc/eks/sandbox-image.service: No such file or directory (os error 2)
ERROR eksnode::commands::validate: /etc/systemd/system/kubelet.service: No such file or directory (os error 2)
```

### Kubelet

```sh
journalctl -u kubelet | grep -e error -e warning

Sep 17 13:12:49 ip-10-0-4-247.ec2.internal kubelet[1814]: Flag --container-runtime-endpoint has been deprecated, This parameter should be set via the config file specified by the Kubelet's --config >
Sep 17 13:12:49 ip-10-0-4-247.ec2.internal kubelet[1814]: Flag --pod-infra-container-image has been deprecated, will be removed in a future release. Image garbage collector will get sandbox image in>
Sep 17 13:12:49 ip-10-0-4-247.ec2.internal kubelet[1814]: Flag --cloud-provider has been deprecated, will be removed in 1.25 or later, in favor of removing cloud provider code from Kubelet.
```

`--pod-infra-container-image string     Default: registry.k8s.io/pause:3.9`
    Specified image will not be pruned by the image garbage collector. CRI implementations have their own configuration to set this image. (DEPRECATED: will be removed in 1.27. Image garbage collector will get sandbox image information from CRI.)

### Containerd

```sh
journalctl -u containerd | grep -e error -e warning

Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.328011138Z" level=info msg="skip loading plugin \"io.containerd.snapshotter.v1.blockfile\"..." error="no scratch file generator: skip plugin" type=io.containerd.snapshotter.v1
Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.328247620Z" level=warning msg="failed to load plugin io.containerd.snapshotter.v1.devmapper" error="devmapper not configured"
Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.328456414Z" level=info msg="skip loading plugin \"io.containerd.snapshotter.v1.zfs\"..." error="path /var/lib/containerd/io.containerd.snapshotter.v1.zfs must be a zfs filesystem to be used with the zfs snapshotter: skip plugin"
Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.328504865Z" level=warning msg="could not use snapshotter devmapper in metadata plugin" error="devmapper not configured"
Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.333142836Z" level=error msg="failed to load cni during init, please check CRI plugin status before setting up network for pods" error="cni config load failed: no network config found in /etc/cni/net.d: cni plugin not initialized: failed to load cni config"
Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.333210586Z" level=info msg="skip loading plugin \"io.containerd.tracing.processor.v1.otlp\"..." error="no OpenTelemetry endpoint: skip plugin" type=io.containerd.tracing.processor.v1
Sep 17 17:34:40 ip-10-0-5-24.ec2.internal containerd[5231]: time="2023-09-17T17:34:40.333234637Z" level=info msg="skipping tracing processor initialization (no tracing plugin)" error="no OpenTelemetry endpoint: skip plugin"
```

## Troubleshooting / Debugging

```sh
systemctl -u containerd | grep -e error -e warning

# Containerd ops https://github.com/containerd/containerd/blob/main/docs/ops.md

# Info on system
nerdctl info
```

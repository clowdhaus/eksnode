# TODO
- [ ] https://github.com/awslabs/amazon-eks-ami/issues/951
  - [ ] https://github.com/awslabs/amazon-eks-ami/issues/478
- [ ] https://github.com/containerd/containerd/issues/6964#issuecomment-1132580240
- [ ] Add log collection functionality
- [ ] Add validate functionality
  - Will need to write a config file to use for validation - will capture the build-spec and join-spec so that when running `eksnode validate`, the default behavior will be to load this file to infer how the node was intended to be setup
- [ ] Enable STIG support
- [ ] Enable FedRAMP support
- [ ] Add reporting for CIS, STIG, FedRAMP compliance with OpenSCAP
- [ ] Add functionality to generate SBOM; both locally and remotely in S3 for use in AMI pipeline
- [ ] Add support for profiles - a profile is a set of customizations to optimize for a particular workload (Spark, ML training, etc.)
  - Compare with <https://github.com/kubernetes/kops/blob/master/nodeup/pkg/model/sysctls.go>

## Containerd/Kubelet

- [ ] Merge default containerd config on host with config provided by users, if one is provided
- [ ] [Drop-in directory for kubelet configuration files](https://kubernetes.io/docs/tasks/administer-cluster/kubelet-config-file/#kubelet-conf-d)

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

(More from the aspect of - how does supporting this plus other similar methods affect the design of `eksnode`?)

- [ ] [WebAssembly and containerd: How it works](https://nigelpoulton.com/webassembly-and-containerd-how-it-works/)
  - [WebAssembly on Kubernetes: The ultimate hands-on guide](https://nigelpoulton.com/webassembly-on-kubernetes-ultimate-hands-on/)
  - [WebAssembly on Kubernetes: everything you need to know](https://nigelpoulton.com/webassembly-on-kubernetes-everything-you-need-to-know/)

"Warning!! The following files are not covered by default logrotate settings ensure they match site policy\"\n\"/etc/logrotate.d/dnf, /etc/logrotate.d/btmp, /etc/logrotate.d/wtmp, /etc/logrotate.d/aide, /etc/logrotate.d/firewalld\"

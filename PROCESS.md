# Process

Captures changes made during AMI creation (static) versus instance creation (dynamic).

## AMI

### Linux

1. Install `eksnode` in `/usr/bin`
2. Install `iptables-legacy` - required by kubelet
3. Install `awscli`
4. Install `amazon-ssm-agent`
5. Remove `ec2-net-utils` - interferes with the route setup on the instance
6. Disable weak SSH ciphers
7. Create(update) `/etc/logrotate.conf` file

### Containerd

8. Create `/etc/eks/` directory
9. Create containerd systemd `/etc/systemd/system/runtime.slice`
10. Install `runc`
11. Install `containerd`
12. Create `/etc/eks/containerd` directory
13. Create `/etc/systemd/system/containerd.service.d` directory
14. Create `/etc/modules-load.d/containerd.conf` file
  - Testing: Verify that the br_netfilter, overlay modules are loaded by running the following commands:
    - `lsmod | grep br_netfilter`
    - `lsmod | grep overlay`
  - https://kubernetes.io/docs/setup/production-environment/container-runtimes/
15. Create `/etc/sysctl.d/99-kubernetes-cri.conf` file
  - Similar to 11 - https://kubernetes.io/docs/setup/production-environment/container-runtimes/
16. Cache local images
  - Verify with `ctr image ls`
17. Create `/etc/containerd/config.toml` file (IF cached?)
18. Create `/etc/systemd/system/sandbox-image.service` file (IF cached?)
  - TODO: duplicate of runtime creation?!
  - `chown root:root /etc/systemd/system/sandbox-image.service`
  - `systemctl enable containerd sandbox-image`
19. Cache local image `pause`
20. Cache local images for `kube-proxy`, `vpc-cni`, `vpc-cni-init` - both default version and latest version
  - Note: `kube-proxy-minimal` is only valid on 1.24+
  - Lots of logic in caching images and regions, FIPs endpoints, etc. TODO - should this be in `eksnode`?

### Kubelet

21. Create `/var/lib/kubelet` directory
22. Download `kubelet` binary & checksum from AWS S3 bucket
  - Verify checksum and move to `/usr/bin/`
23. Create `/etc/kubernetes/kubelet` directory
24. Create `/etc/systemd/system/kubelet.service.d` directory

### Kubernetes/EKS

25. Create `/etc/logrotate.d/kube-proxy` directory
26. Create `/etc/kubernetes/manifests` directory - TODO - what is this used for?
27. Create `/var/lib/kubernetes` directory
28. Create `/opt/cni/bin` directory
29. Download CNI binary & checksum from GitHub
  - Move to `/opt/cni/bin/`
30. Download `aws-iam-authenticator` binary & checksum from S3 bucket
  - Verify checksum and move to `/usr/bin/`
31. Create `/etc/eks/image-credential-provider` directory
32. Download `ecr-credential-provider` binary from S3 bucket
  - Move to `/etc/eks/image-credential-provider/`
33. Create `/etc/eks/release` containing build metadata
34. Add to `/etc/sysctl.d/99-amazon.conf` file for "protectKernelDefaults=true"
35. Add to `/etc/sysctl.conf` file for sysctl settings
  - [Reference](https://github.com/kubernetes/kops/blob/master/nodeup/pkg/model/sysctls.go)
36. TODO - `/etc/eks/bootstrap.sh` psuedo bootstrap file for backwards compatibility
37. Create `/etc/eks/log-collector-script/` file
  - TODO: replace this with `eksnode`

## Instance

1. Create `/etc/eks/image-credential-provider/config.json` file
2. Create `/etc/eks/containerd/containerd-config.toml` - update sandbox image + user settings
3. Create `/etc/eks/containerd/kubelet-containerd.service` - update kubelet args and kubelet extra args
4. Create `/etc/eks/containerd/sandbox-image.service` - update sandbox image
  - TODO - figure out how to handle `pull-image.sh` and `pull-sandbox-image.sh` scripts used by sandbox image service
5. Create `/var/lib/kubelet/kubeconfig`
  - `chown root:root /var/lib/kubelet/kubeconfig`
6. Create `/etc/systemd/system/kubelet.service`
  - `chown root:root /etc/systemd/system/kubelet.service`
7. Create `/etc/kubernetes/kubelet/kubelet-config.json`
  - `chown root:root /etc/kubernetes/kubelet/kubelet-config.json`
8. `sudo systemctl daemon-reload`
9. For (containerd, kubelet, ???):
  - `systemctl enable ${THING}`
  - `systemctl start ${THING}`

## TODO

- Double check against https://github.com/containerd/containerd/blob/main/docs/getting-started.md
- Enable/disable swap - https://github.com/kubernetes/enhancements/tree/master/keps/sig-node/2400-node-swap
  - https://github.com/kubernetes/enhancements/issues/2400
- sudo ufw disable ?
- iptables-legacy ? - https://kubernetes.io/blog/2022/09/07/iptables-chains-not-api/
- What about containerd service [file](https://raw.githubusercontent.com/containerd/containerd/main/containerd.service)?

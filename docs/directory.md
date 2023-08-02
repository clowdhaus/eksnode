# Kubernetes Directory Structure

```sh
/etc/
  ├─ cni
  │   └─ [0700] net.d/                      # Folder containing CNI configuration files
  │       └─ [0644] 10-aws.conflist         # CNI configuration file for AWS VPC CNI ( who created this, VPC CNI?)
  ├─ eks
  │   ├─ [0755] bootstrap.sh # Psuedo script for joining node to cluster
  │   ├─ containerd/
  │   │   ├─ [0644] containerd-cached-pause-config.toml ???
  │   │   ├─ [0644] containerd-config.toml
  │   │   ├─ [0644] kubelet-containerd.service
  │   │   ├─ [0755] pull-image.sh
  │   │   ├─ [0755] pull-sandbox-image.sh
  │   │   └─ [0644] sandbox-image.service
  │   ├─ [0644] eni-max-pods.txt
  │   ├─ image-credential-provider/
  │   │   ├─ [0644] config.json
  │   │   └─ [0755] ecr-credential-provider
  │   ├─ [0644] iptables-restore.service
  │   ├─ [0755] max-pods-calculator.sh # Psuedo script for calculating max pods
  │   └─ [0644] release
  ├─ kubernetes/
  │   ├─ kubelet/
  │   │   └─ [0644] kubelet-config.json   # kubelet configuration file
  │   ├─ manifests/
  │   │   └─ ...
  │   └─ pki/                             # Public Key Infrastructure for Kubernetes
  │       └─ [0644] ca.crt                # Cluster Certificate Authority certificate
  └─ [0644] logrotate.conf
    └─ logrotate.d/
        ├─ [0644] kube-proxy # kube-proxy logrotate configuration file
        └─ ...

/var/
  └─ lib/
      ├─ kubernetes/ # TODO - should this be removed?
      └─ kubelet/
          ├─ [0644] kubeconfig
          └─ ... # Other files and directories created by kubelet

/opt/cni/bin/                        # Container Networking Interface (CNI) binaries

    -rwxr-xr-x 1 root root 11157504 Jul 31 22:41 aws-cni
    -rwxr-xr-x 1 root root    24740 Jul 31 22:41 aws-cni-support.sh
    -rwxr-xr-x 1 root root  3859475 Jul 31 22:41 bandwidth
    -rwxr-xr-x 1 root root  4299004 Jan 16  2023 bridge
    -rwxr-xr-x 1 root root 10167415 Jan 16  2023 dhcp
    -rwxr-xr-x 1 root root  3986082 Jan 16  2023 dummy
    -rwxr-xr-x 1 root root  4714496 Jul 31 22:41 egress-v4-cni
    -rwxr-xr-x 1 root root  4385098 Jan 16  2023 firewall
    -rwxr-xr-x 1 root root  3870731 Jan 16  2023 host-device
    -rwxr-xr-x 1 root root  3287319 Jul 31 22:41 host-local
    -rwxr-xr-x 1 root root  3999593 Jan 16  2023 ipvlan
    -rwxr-xr-x 1 root root  3353028 Jul 31 22:41 loopback
    -rwxr-xr-x 1 root root  4029261 Jan 16  2023 macvlan
    -rwxr-xr-x 1 root root  3746163 Jul 31 22:41 portmap
    -rwxr-xr-x 1 root root  4161070 Jan 16  2023 ptp
    -rwxr-xr-x 1 root root  3550152 Jan 16  2023 sbr
    -rwxr-xr-x 1 root root  2845685 Jan 16  2023 static
    -rwxr-xr-x 1 root root  3437180 Jan 16  2023 tuning
    -rwxr-xr-x 1 root root  3993252 Jan 16  2023 vlan
    -rwxr-xr-x 1 root root  3586502 Jan 16  2023 vrf

/usr/local/sbin/runc # Al2 uses yum install so this doesn't exist on Al2

/etc/systemd/system/runtime.slice
/etc/systemd/system/containerd.service.d
/etc/systemd/system/kubelet.service.d

    drwxr-xr-x  2 root     root       57 Jul 19 23:38 basic.target.wants
    drwxr-xr-x  2 root     root      119 Jul 19 23:38 cloud-init.target.wants
    -rw-r--r--  1 root     root      138 Jul 31 22:41 configure-clocksource.service
    drwxr-xr-x  2 root     root       65 Jul 31 22:41 containerd.service.d
    lrwxrwxrwx  1 root     root       40 Jul 19 23:38 default.target -> /usr/lib/systemd/system/graphical.target
    drwxr-xr-x  2 root     root       87 Jul 19 23:38 default.target.wants
    drwxr-xr-x  2 root     root       32 Jul 19 23:38 getty.target.wants
    -rw-r--r--  1 root     root      850 Jul 31 22:41 kubelet.service
    drwxr-xr-x  2 root     root       68 Jul 31 22:41 kubelet.service.d
    drwxr-xr-x  2 root     root       78 Jul 19 23:38 local-fs.target.wants
    drwxr-xr-x  2 root     root     4096 Jul 31 22:41 multi-user.target.wants
    drwxr-xr-x  2 root     root       31 Jul 28 04:17 remote-fs.target.wants
    -rw-r--r--  1 ec2-user ec2-user  116 Jul 28 04:15 runtime.slice
    -rw-r--r--  1 root     root      276 Jul 31 22:41 sandbox-image.service
    drwxr-xr-x  2 root     root       51 Jul 28 04:18 sockets.target.wants
    -rw-r--r--  1 root     root      295 Jul 31 22:41 sys-fs-bpf.mount
    drwxr-xr-x  2 root     root      254 Jul 28 04:18 sysinit.target.wants
    drwxr-xr-x  2 root     root       44 Jul 19 23:38 system-update.target.wants

- /etc/systemd/system/kubelet.service.d/10-kubelet-args.conf
    [Service]
    Environment='KUBELET_ARGS=--node-ip=10.0.14.36 --pod-infra-container-image=602401143452.dkr.ecr.us-east-1.amazonaws.com/eks/pause:3.5 --v=2 --hostname-override=ip-10-0-14-36.ec2.internal --cloud-provider=external'

- /etc/systemd/system/kubelet.service.d/30-kubelet-extra-args.conf
    [Service]
    Environment='KUBELET_EXTRA_ARGS=--node-labels=eks.amazonaws.com/sourceLaunchTemplateVersion=1,eks.amazonaws.com/nodegroup-image=ami-0bc4534a93057f9fb,eks.amazonaws.com/capacityType=ON_DEMAND,eks.amazonaws.com/nodegroup=default-2023073120544194420000000f,eks.amazonaws.com/sourceLaunchTemplateId=lt-0be6ae398df1b0f70 --max-pods=29'

- /etc/systemd/system/containerd.service.d/00-runtime-slice.conf
    [Service]
    Slice=runtime.slice

- /etc/systemd/system/containerd.service.d/10-compat-symlink.conf
    [Service]
    ExecStartPre=/bin/ln -sf /run/containerd/containerd.sock /run/dockershim.sock

/etc/modules-load.d/containerd.conf
/etc/sysctl.d/99-kubernetes-cri.conf
/etc/sysctl.d/99-amazon.conf
/etc/sysctl.conf

/usr/local/bin/containerd/containerd # Not there on AL2 since installed by yum

[0755] (ec2-user:ec2-user) /usr/bin/aws-iam-authenticator
[0755] (ec2-user:ec2-user) /usr/bin/kubelet
```

# Container Images

## Caching

The AMI build process supports caching commonly used images on the resulting AMI in order to improve the time it takes
for a node to reach a `Ready` state.

By default, this project caches the following images on the generated AMI:

- `<ECR-ENDPOINT>/eks/kube-proxy:<default and latest>-eksbuild.<BUILD_VERSION>`
- `<ECR-ENDPOINT>/eks/kube-proxy:<default and latest>-minimal-eksbuild.<BUILD_VERSION>`
- `<ECR-ENDPOINT>/eks/pause:3.5`
- `<ECR-ENDPOINT>/amazon-k8s-cni-init:<default and latest>`
- `<ECR-ENDPOINT>/amazon-k8s-cni:<default and latest>`

## Sandbox Image

When building the AMI, its recommended to cache the sandbox image on the AMI so that its available when the instance starts up.
However, some

This sandbox image value is used in multiple locations and processes:

- Set in `/etc/containerd/config.toml` - this is the main priority for the pause container image since it is used by containerd
- Set in `/etc/systemd/system/sandbox-image.service` which ensures the image has been pulled,  if not already present, when the instance launches
- Set in `/etc/systemd/system/kubelet.service.d/10-kubelet-args.conf` to ensure that kubelet does not garbage collect this image
  - TODO - this is removed in 1.30 <https://github.com/kubernetes/kubernetes/blob/3ac83f528dde9d6f37f0ca164d5642226f2380a7/cmd/kubelet/app/server.go#L203-L204>
- It is used in the AMI build process by `eksnode` to cache the image on the AMI

There is a tradeoff between caching the image for faster startup times and allowing users to specify a different container image at the time

## Caveats

- AMIs should be built per region instead of copying AMI to additional regions
  - The repository URI and service endpoints are set based on the region where the AMI is built

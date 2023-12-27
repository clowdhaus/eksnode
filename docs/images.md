# Container Images

## Image Caching

The AMI build process supports caching commonly used images on the AMI in order to improve the time it takes
for a node to reach a `Ready` state.

By default, this project caches the following images on the generated AMI:

- `<ECR-ENDPOINT>/eks/kube-proxy:<default and latest>-eksbuild.<BUILD_VERSION>`
- `<ECR-ENDPOINT>/eks/kube-proxy:<default and latest>-minimal-eksbuild.<BUILD_VERSION>`
- `<ECR-ENDPOINT>/eks/pause:3.8`
- `<ECR-ENDPOINT>/amazon-k8s-cni-init:<default and latest>`
- `<ECR-ENDPOINT>/amazon-k8s-cni:<default and latest>`

!!! note "Tagging cached images for multi-region support"

    The images listed above are also tagged with each region (ECR endpoint) in the partition the AMI is built within, since images are often built in one region and copied to others within the same partition. This results in a long list of images when running `nerdctl images` on a node. You can think of the regional tags as an alias for the image; they do not incur any additional storage overhead and only the image the tags point to will consume storage on the node. The images cached on the AMI consume roughly ~1.0 GiB of storage.

## Sandbox Image

The sandbox image value is used in multiple locations and processes:

- Set in `/etc/containerd/config.toml` - this is the main priority for the pause container image since it is used by `containerd`
- Set in `/etc/systemd/system/sandbox-image.service` - ensures the image has been pulled, if not already present, when the instance starts
- Set in `/etc/kubernetes/kubelet/kubelet-config.json` under `pod-infra-container-image` to ensure that kubelet does not garbage collect the image
<!-- TODO - This is removed in 1.30 <https://github.com/kubernetes/kubernetes/blob/3ac83f528dde9d6f37f0ca164d5642226f2380a7/cmd/kubelet/app/server.go#L203-L204> -->
- It is used in the AMI build process by `eksnode` to cache the image on the AMI

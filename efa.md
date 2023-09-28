# EFA Installer README

The EFA installer will install the userspace libraries and kernel
modules necessary to use the Elastic Fabric Adapter (EFA) on EC2
instances.  Additional software may be required, such as Intel MPI or
the nccl-ofi-plugin for machine learning applications using NCCL.

## Public Documentation

Refer to the [official documentation](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/efa-start.html) for official instructions to use the installer.

- [Supported instance types](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/efa.html#efa-instance-types)
- [Supported operating systems](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/efa.html#efa-os)

## Installer Usage

The `efa_installer.sh` script managing installing operating system
specific packages.  Most software is installed in `/opt/amazon/efa/`,
except for configuration packages that must place files in `/etc` and
the rdma-core package, which is designed to only install in `/usr`.
The following command line arguments are available to
`efa_installer.sh`:

- `--debug-pacakges`: Install the debug packages next to standard
  packages.
- `--uninstall`: Uninstall all packages installed by the EFA
  installer.
- `--yes`: Assume yes to any prompts and run the installer without
  prompting for verification.
- `--no-verify`: Skip running EFA functionality verification tests
  at end of installer.  Useful if building an AMI on an instance
  type which does not support EFA.
- `--skip-limit-conf`: Skip installing configurations to change the
  default limits for locked pages.
- `--skip-kmod`: Do not install the EFA kernel module while
  installing user-space packages.  Useful for installing EFA
  libraries in containers.
- `--enable-gdr`: Enable GPUDirect RDMA support.  This will result
  in the installation of a kernel module which can only be loaded if
  the NVIDIA driver stack is already loaded, so limits the kernel
  module to only working on instance types with NVIDIA GPUs.
- `--minimal`: Only install the kernel module and rdma-core.  Do not
  install Open MPI or Libfabric.

## Installed Packages

- dkms
- rdma-core
- libfabric
- Open MPI
- efa-config
- efa-profile
- efa kernel module

Other system packages may be installed as dependencies of these packages.

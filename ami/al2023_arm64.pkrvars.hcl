ami_description  = "Amazon EKS arm64 AL2023 image"
instance_type    = "c6g.large"
cpu_architecture = "arm64"

ami_block_device_mappings = [
  {
    device_name = "/dev/xvda"
  },
]

launch_block_device_mappings = [
  {
    device_name = "/dev/xvda"
  },
]

# TODO - figure out SSM access
associate_public_ip_address = true

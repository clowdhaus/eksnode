ami_description  = "Amazon EKS x86_64/amd64 AL2023 image"
instance_type    = "c6i.large"
cpu_architecture = "x86_64"

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

ami_description  = "Amazon EKS arm64 AL2023 image"
instance_type    = "c6g.large"
cpu_architecture = "arm64"

# Amazon Linux 2023 minimal does not come with SSM agent installed by default
user_data_file = "./files/ssm_user_data.sh"

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

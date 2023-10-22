ami_name_prefix  = "amazon-eks-gpu"
ami_description  = "Amazon EKS x86_64/amd64 GPU AL2023 image"
instance_type    = "g4dn.8xlarge"
cpu_architecture = "x86_64"
launch_block_device_mappings = [
  {
    device_name           = "/dev/xvda"
    volume_size           = 8
    volume_type           = "gp3"
    delete_on_termination = true
  },
]

# Amazon Linux 2023 minimal does not come with SSM agent installed by default
user_data_file = "./files/ssm_user_data.sh"

install_nvidia_driver = true
install_efa_driver    = true

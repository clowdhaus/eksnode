ami_name_prefix  = "amazon-eks-neuron"
ami_description  = "Amazon EKS x86_64/amd64 Neuron AL2023 image"
instance_type    = "trn1.2xlarge"
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

install_neuron = true
install_efa    = true

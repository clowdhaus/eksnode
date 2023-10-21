ami_description  = "Amazon EKS x86_64/amd64 AL2023 image"
instance_type    = "c6i.large"
cpu_architecture = "x86_64"

# Amazon Linux 2023 minimal does not come with SSM agent installed by default
user_data_file = "./files/ssm_user_data.sh"

ansible_roles = ["harden"]

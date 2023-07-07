
################################################################################
# EC2 Instance
################################################################################

locals {
  arch = "x86_64"
}

data "aws_ssm_parameter" "al2023" {
  for_each = toset(["x86_64", "arm64"])

  name = "/aws/service/ami-amazon-linux-latest/al2023-ami-minimal-kernel-default-${each.value}"
}

data "aws_ssm_parameter" "eks" {
  for_each = toset(["1.24"])

  name = "/aws/service/eks/optimized-ami/${each.value}/amazon-linux-2/recommended/image_id"
}

# Only SSH is permitted to connect to AL2023
module "key_pair" {
  source  = "terraform-aws-modules/key-pair/aws"
  version = "~> 2.0"

  key_name           = local.name
  create_private_key = true

  tags = module.tags.tags
}

resource "local_sensitive_file" "key_pair" {
  content         = module.key_pair.private_key_pem
  filename        = "${path.module}/${module.key_pair.key_pair_name}.pem"
  file_permission = "0400"
}

module "ec2" {
  source  = "terraform-aws-modules/ec2-instance/aws"
  version = "~> 5.0"

  name = local.name

  ami                    = data.aws_ssm_parameter.al2023[local.arch].value
  instance_type          = "t3.large"
  availability_zone      = element(module.vpc.azs, 0)
  subnet_id              = element(module.vpc.public_subnets, 0)
  vpc_security_group_ids = [module.security_group.security_group_id]

  create_iam_instance_profile = true
  iam_role_policies = {
    # Node permissions
    AmazonEKSWorkerNodePolicy          = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
    AmazonEC2ContainerRegistryReadOnly = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
    AmazonEKS_CNI_Policy               = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  }

  associate_public_ip_address = true
  key_name                    = module.key_pair.key_pair_name

  user_data                   = <<-EOT
    #!/bin/bash

    dnf install rust cargo -y
  EOT
  user_data_replace_on_change = true

  metadata_options = {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 2
  }

  root_block_device = [
    {
      encrypted   = true
      volume_type = "gp3"
      volume_size = 64
    },
  ]

  tags = module.tags.tags
}

module "ec2_eks_24" {
  source  = "terraform-aws-modules/ec2-instance/aws"
  version = "~> 5.0"

  name = "${local.name}-eks-24"

  ami                    = data.aws_ssm_parameter.eks["1.24"].value
  instance_type          = "t3.large"
  availability_zone      = element(module.vpc.azs, 0)
  subnet_id              = element(module.vpc.public_subnets, 0)
  vpc_security_group_ids = [module.security_group.security_group_id]

  create_iam_instance_profile = true
  iam_role_policies = {
    # Node permissions
    AmazonEKSWorkerNodePolicy          = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
    AmazonEC2ContainerRegistryReadOnly = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
    AmazonEKS_CNI_Policy               = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"
  }

  associate_public_ip_address = true
  key_name                    = module.key_pair.key_pair_name

  # user_data                   = <<-EOT
  #   #!/bin/bash

  #   dnf install rust cargo -y
  # EOT
  # user_data_replace_on_change = true

  metadata_options = {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 2
  }

  root_block_device = [
    {
      encrypted   = true
      volume_type = "gp3"
      volume_size = 64
    },
  ]

  tags = module.tags.tags
}

################################################################################
# VPC
################################################################################

module "vpc" {
  source  = "terraform-aws-modules/vpc/aws"
  version = "~> 5.0"

  name = local.name
  cidr = local.vpc_cidr

  azs             = local.azs
  private_subnets = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 4, k)]
  public_subnets  = [for k, v in local.azs : cidrsubnet(local.vpc_cidr, 8, k + 48)]

  tags = module.tags.tags
}

data "http" "myip" {
  url = "http://ipv4.icanhazip.com"
}

module "security_group" {
  source  = "terraform-aws-modules/security-group/aws"
  version = "~> 5.0"

  name        = local.name
  description = "Security group for example usage with EC2 instance"
  vpc_id      = module.vpc.vpc_id

  ingress_with_cidr_blocks = [
    {
      from_port   = 22
      to_port     = 22
      protocol    = "tcp"
      description = "SSH access"
      cidr_blocks = "${chomp(data.http.myip.body)}/32"
    },
  ]
  egress_rules = ["all-all"]

  tags = module.tags.tags
}

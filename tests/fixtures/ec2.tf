
################################################################################
# EC2 Instance
################################################################################

data "aws_ssm_parameter" "al2023" {
  for_each = toset(["x86_64", "arm64"])

  name = "/aws/service/ami-amazon-linux-latest/al2023-ami-minimal-kernel-default-${each.value}"
}

output "ami" {
  value = nonsensitive(data.aws_ssm_parameter.al2023["x86_64"].value)
}

module "ec2_complete" {
  source  = "terraform-aws-modules/ec2-instance/aws"
  version = "~> 5.0"

  name = local.name

  ami                    = data.aws_ssm_parameter.al2023["x86_64"].value
  instance_type          = "t3.micro"
  availability_zone      = element(module.vpc.azs, 0)
  subnet_id              = element(module.vpc.public_subnets, 0)
  vpc_security_group_ids = [module.security_group.security_group_id]

  iam_role_policies = {
    # Node permissions
    AmazonEKSWorkerNodePolicy          = "arn:aws:iam::aws:policy/AmazonEKSWorkerNodePolicy"
    AmazonEC2ContainerRegistryReadOnly = "arn:aws:iam::aws:policy/AmazonEC2ContainerRegistryReadOnly"
    AmazonEKS_CNI_Policy               = "arn:aws:iam::aws:policy/AmazonEKS_CNI_Policy"

    # Remote SSM access
    AmazonSSMManagedInstanceCore = "arn:aws:iam::aws:policy/AmazonSSMManagedInstanceCore"
  }

  # user_data_base64            = base64encode(local.user_data)
  # user_data_replace_on_change = true

  metadata_options = {
    http_endpoint               = "enabled"
    http_tokens                 = "required"
    http_put_response_hop_limit = 2
  }

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

module "security_group" {
  source  = "terraform-aws-modules/security-group/aws"
  version = "~> 5.0"

  name        = local.name
  description = "Security group for example usage with EC2 instance"
  vpc_id      = module.vpc.vpc_id

  egress_rules = ["all-all"]

  tags = module.tags.tags
}

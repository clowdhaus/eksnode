locals {
  timestamp = regex_replace(timestamp(), "[- TZ:]", "")

  target_ami_name = "${var.ami_name_prefix}-${var.eks_version}-${local.timestamp}"
}

data "amazon-parameterstore" "this" {
  name   = "/aws/service/ami-amazon-linux-latest/al2023-ami-minimal-kernel-6.1-${var.source_ami_arch}"
  region = var.aws_region
}

source "amazon-ebs" "this" {
  ami_block_device_mappings {
    delete_on_termination = true
    device_name           = "/dev/sdb"
    volume_size           = var.data_volume_size
    volume_type           = "gp3"
  }

  ami_description         = var.ami_description
  ami_name                = local.target_ami_name
  ami_virtualization_type = "hvm"
  instance_type           = var.instance_type

  launch_block_device_mappings {
    delete_on_termination = true
    device_name           = "/dev/xvda"
    volume_size           = var.root_volume_size
    volume_type           = "gp3"
  }

  launch_block_device_mappings {
    delete_on_termination = true
    device_name           = "/dev/xvdb"
    volume_size           = var.data_volume_size
    volume_type           = "gp3"
  }

  region = var.aws_region

  run_tags = {
    Name = local.target_ami_name
  }

  source_ami = data.amazon-parameterstore.this.value
  subnet_id = var.subnet_id

  communicator  = "ssh"
  ssh_interface = "public_dns"
  ssh_username = var.ssh_username
  associate_public_ip_address = true

  // temporary_iam_instance_profile_policy_document {
  //   Statement {
  //     Action   = ["*"]
  //     Effect   = "Allow"
  //     Resource = ["*"]
  //   }
  //   Version = "2012-10-17"
  // }

  tags = {
    os_version        = "Amazon Linux 2023"
    source_image_name = "{{ .SourceAMIName }}"
    ami_type          = "al2023"
  }
}

build {
  sources = ["source.amazon-ebs.this"]

  provisioner "ansible" {
    playbook_file = "./playbooks/al2023_playbook.yaml"
  }

  // provisioner "shell" {
  //   execute_command   = "echo 'packer' | {{ .Vars }} sudo -S -E bash -eux '{{ .Path }}'"
  //   expect_disconnect = true
  //   pause_after       = "15s"
  //   script            = "scripts/update.sh"
  // }

  // provisioner "shell" {
  //   execute_command = "echo 'packer' | {{ .Vars }} sudo -S -E bash -eux '{{ .Path }}'"

  //   environment_vars = [
  //     "HTTP_PROXY=${var.http_proxy}",
  //     "HTTPS_PROXY=${var.https_proxy}",
  //     "NO_PROXY=${var.no_proxy}",
  //   ]

  //   expect_disconnect = true
  //   pause_after       = "15s"
  //   scripts = [
  //     "scripts/partition-disks.sh",
  //     "scripts/configure-proxy.sh",
  //     "scripts/configure-containers.sh",
  //   ]
  // }

  // provisioner "shell" {
  //   execute_command = "echo 'packer' | {{ .Vars }} sudo -S -E bash -eux '{{ .Path }}'"

  //   scripts = [
  //     "scripts/cleanup.sh",
  //   ]
  // }
}

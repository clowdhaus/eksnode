locals {
  timestamp = regex_replace(timestamp(), "[- TZ:]", "")

  target_ami_name = "${var.ami_name_prefix}-${var.eks_version}-${local.timestamp}"
}

data "amazon-parameterstore" "this" {
  name = "/aws/service/ami-amazon-linux-latest/al2023-ami-minimal-kernel-default-${var.source_ami_arch}"
}

source "amazon-ebs" "this" {
  ami_block_device_mappings {
    delete_on_termination = true
    device_name           = "/dev/sdb"
    volume_size           = var.data_volume_size
    volume_type           = "gp2"
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

  source_ami   = data.amazon-parameterstore.this.value
  ssh_pty      = true
  ssh_username = var.source_ami_ssh_user
  subnet_id    = var.subnet_id

  tags = {
    os_version        = "Amazon Linux 2"
    source_image_name = "{{ .SourceAMIName }}"
    ami_type          = "al2023"
  }
}

build {
  sources = ["source.amazon-ebs.this"]

  provisioner "shell" {
    execute_command   = "echo 'packer' | {{ .Vars }} sudo -S -E bash -eux '{{ .Path }}'"
    expect_disconnect = true
    pause_after       = "15s"
    script            = "scripts/update.sh"
  }

  provisioner "shell" {
    execute_command = "echo 'packer' | {{ .Vars }} sudo -S -E bash -eux '{{ .Path }}'"

    environment_vars = [
      "HTTP_PROXY=${var.http_proxy}",
      "HTTPS_PROXY=${var.https_proxy}",
      "NO_PROXY=${var.no_proxy}",
    ]

    expect_disconnect = true
    pause_after       = "15s"
    scripts = [
      "scripts/partition-disks.sh",
      "scripts/configure-proxy.sh",
      "scripts/configure-containers.sh",
    ]
  }

  provisioner "shell" {
    execute_command = "echo 'packer' | {{ .Vars }} sudo -S -E bash -eux '{{ .Path }}'"

    scripts = [
      "scripts/cis-benchmark.sh",
      "scripts/cis-docker.sh",
      "scripts/cis-eks.sh",
      "scripts/cleanup.sh",
    ]
  }
}
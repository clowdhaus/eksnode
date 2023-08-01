output "ssh_connect_al2023" {
  description = "SSH command to connect to remote host created"
  value       = "ssh -i ${module.key_pair.key_pair_name}.pem ec2-user@${module.ec2.public_dns}"
}

output "ssh_connect_eks_al2" {
  description = "SSH command to connect to remote host created"
  value       = "ssh -i ${module.key_pair.key_pair_name}.pem ec2-user@${module.ec2_eks_al2.public_dns}"
}

output "ansible_connect" {
  description = "Connect via Ansible"
  value       = "ansible-playbook al2023_playbook.yaml -i ansible_hosts --user ec2-user --key-file ${local_sensitive_file.key_pair.filename} -e key=${local_sensitive_file.pub_key_pair.filename}"
}

output "private_subnets" {
  description = "Private subnets"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "Public subnets"
  value       = module.vpc.public_subnets
}

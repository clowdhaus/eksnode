output "ssh_connect_al2023" {
  description = "SSH command to connect to remote host created"
  value       = "ssh -i ${module.key_pair.key_pair_name}.pem ec2-user@${module.ec2.public_dns}"
}

output "ssh_connect_eks" {
  description = "SSH command to connect to remote host created"
  value       = "ssh -i ${module.key_pair.key_pair_name}.pem ec2-user@${module.ec2_eks_24.public_dns}"
}

output "scp_cmd" {
  description = "SCP command to copy files to remote host"
  value       = "scp -i ${module.key_pair.key_pair_name}.pem -r ../../eksnode ec2-user@${module.ec2.public_dns}:~/"
}

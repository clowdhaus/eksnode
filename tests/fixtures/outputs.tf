output "ssh_connect" {
  description = "SSH command to connect to remote host created"
  value       = "ssh -i ${module.key_pair.key_pair_name}.pem ec2-user@${module.ec2.public_dns}"
}

output "scp_cmd" {
  description = "SCP command to copy files to remote host"
  value       = "scp -i ${module.key_pair.key_pair_name}.pem -r ../../eksami ec2-user@${module.ec2.public_dns}:~/"
}

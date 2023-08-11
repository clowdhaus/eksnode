# Amazon Machine Image (AMI) Packer Configuration(s)

```sh
packer init .
packer build -var-file=al2023_x86_64.pkrvars.hcl -var 'subnet_id=subnet-xxx' .
```

<!-- BEGIN_TF_DOCS -->
## Requirements

No requirements.

## Providers

| Name | Version |
|------|---------|
| <a name="provider_amazon-parameterstore"></a> [amazon-parameterstore](#provider\_amazon-parameterstore) | n/a |

## Modules

No modules.

## Resources

| Name | Type |
|------|------|
| [amazon-parameterstore_amazon-parameterstore.this](https://registry.terraform.io/providers/hashicorp/amazon-parameterstore/latest/docs/data-sources/amazon-parameterstore) | data source |

## Inputs

| Name | Description | Type | Default | Required |
|------|-------------|------|---------|:--------:|
| <a name="input_ami_description"></a> [ami\_description](#input\_ami\_description) | The description to use when creating the AMI | `string` | `"Amazon EKS AL2023 image"` | no |
| <a name="input_ami_name_prefix"></a> [ami\_name\_prefix](#input\_ami\_name\_prefix) | The prefix to use when creating the AMI name. i.e. - `<ami_name_prefix>-<eks_version>-<timestamp>` | `string` | `"amazon-eks-node"` | no |
| <a name="input_aws_region"></a> [aws\_region](#input\_aws\_region) | Region where AMI will be created | `string` | `"us-east-1"` | no |
| <a name="input_data_volume_size"></a> [data\_volume\_size](#input\_data\_volume\_size) | Size of the AMI data EBS volume | `number` | `50` | no |
| <a name="input_eks_version"></a> [eks\_version](#input\_eks\_version) | The EKS cluster version associated with the AMI created | `string` | `"1.27"` | no |
| <a name="input_http_proxy"></a> [http\_proxy](#input\_http\_proxy) | The HTTP proxy to set on the AMI created | `string` | `""` | no |
| <a name="input_https_proxy"></a> [https\_proxy](#input\_https\_proxy) | The HTTPS proxy to set on the AMI created | `string` | `""` | no |
| <a name="input_instance_type"></a> [instance\_type](#input\_instance\_type) | The instance type to use when creating the AMI. Note: this should be adjusted based on the `source_ami_arch` provided | `string` | `"c6i.large"` | no |
| <a name="input_no_proxy"></a> [no\_proxy](#input\_no\_proxy) | Disables proxying on the AMI created | `string` | `""` | no |
| <a name="input_root_volume_size"></a> [root\_volume\_size](#input\_root\_volume\_size) | Size of the AMI root EBS volume | `number` | `10` | no |
| <a name="input_source_ami_arch"></a> [source\_ami\_arch](#input\_source\_ami\_arch) | The architecture of the source AMI. Either `x86_64` or `arm64` | `string` | `"x86_64"` | no |
| <a name="input_source_ami_owner"></a> [source\_ami\_owner](#input\_source\_ami\_owner) | The owner of the source AMI | `string` | `"amazon"` | no |
| <a name="input_source_ami_owner_govcloud"></a> [source\_ami\_owner\_govcloud](#input\_source\_ami\_owner\_govcloud) | The owner of the source AMI in the GovCloud region | `string` | `"219670896067"` | no |
| <a name="input_ssh_username"></a> [ssh\_username](#input\_ssh\_username) | The SSH user used when connecting to the AMI for provisioning | `string` | `"ec2-user"` | no |
| <a name="input_subnet_id"></a> [subnet\_id](#input\_subnet\_id) | The subnet ID where the AMI can be created. Required if a default VPC is not present in the `aws_region` | `string` | `null` | no |

## Outputs

No outputs.
<!-- END_TF_DOCS -->

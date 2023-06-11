# The regions queried were:
{{ #each regions as |region| }}
# - {{ region }}
{{ /each }}
#
# Mapping is calculated from AWS EC2 API using the following formula:
# * First IP on each ENI is not used for pods
# * +2 for the pods that use host-networking (AWS CNI and kube-proxy)
#
#   # of ENI * (# of IPv4 per ENI - 1) + 2
#
# Note: only one network card is supported, so use the MaximumNetworkInterfaces
# from the default card if more than one is present
# https://github.com/aws/amazon-vpc-cni-k8s/blob/4bd975383285cc9607f2bde3229bdefe2a44d815/scripts/gen_vpc_ip_limits.go#L162
#
# https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/using-eni.html#AvailableIpPerENI
#
{{ #each instances as |instance| }}
{{ @key }} {{ instance.maximum_pods }}
{{ /each }}
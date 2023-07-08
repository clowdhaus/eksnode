#!/usr/bin/env bash

set -o pipefail
set -o nounset
set -o errexit

function print_help {
  echo "usage: $0 [options] <cluster-name>"
  echo "Bootstraps an instance into an EKS cluster"
  echo ""
  echo "-h,--help print this help"
  echo
  echo "--apiserver-endpoint The EKS cluster API Server endpoint. Only valid when used with --b64-cluster-ca. Bypasses calling \"aws eks describe-cluster\""
  echo "--aws-api-retry-attempts Number of retry attempts for AWS API call (DescribeCluster) (default: 3)"
  echo "--b64-cluster-ca The base64 encoded cluster CA content. Only valid when used with --apiserver-endpoint. Bypasses calling \"aws eks describe-cluster\""
  echo "--cluster-id Specify the id of EKS cluster"
  echo "--containerd-config-file File containing the containerd configuration to be used in place of AMI defaults."
  echo "--dns-cluster-ip Overrides the IP address to use for DNS queries within the cluster. Defaults to 10.100.0.10 or 172.20.0.10 based on the IP address of the primary interface"
  echo "--enable-local-outpost Enable support for worker nodes to communicate with the local control plane when running on a disconnected Outpost. (true or false)"
  echo "--ip-family Specify ip family of the cluster"
  echo "--kubelet-extra-args Extra arguments to add to the kubelet. Useful for adding labels or taints."
  echo "--local-disks Setup instance storage NVMe disks in raid0 or mount the individual disks for use by pods [mount | raid0]"
  echo "--mount-bfs-fs Mount a bpffs at /sys/fs/bpf (default: true, for Kubernetes 1.27+; false otherwise)"
  echo "--pause-container-account The AWS account (number) to pull the pause container from"
  echo "--pause-container-version The tag of the pause container"
  echo "--service-ipv6-cidr ipv6 cidr range of the cluster"
  echo "--use-max-pods Sets --max-pods for the kubelet when true. (default: true)"
}

while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
    -h | --help)
      print_help
      exit 1
      ;;
    --use-max-pods)
      USE_MAX_PODS="$2"
      log "INFO: --use-max-pods='${USE_MAX_PODS}'"
      shift
      shift
      ;;
    --b64-cluster-ca)
      B64_CLUSTER_CA=$2
      log "INFO: --b64-cluster-ca='${B64_CLUSTER_CA}'"
      shift
      shift
      ;;
    --apiserver-endpoint)
      APISERVER_ENDPOINT=$2
      log "INFO: --apiserver-endpoint='${APISERVER_ENDPOINT}'"
      shift
      shift
      ;;
    --kubelet-extra-args)
      KUBELET_EXTRA_ARGS=$2
      log "INFO: --kubelet-extra-args='${KUBELET_EXTRA_ARGS}'"
      shift
      shift
      ;;
    --aws-api-retry-attempts)
      API_RETRY_ATTEMPTS=$2
      log "INFO: --aws-api-retry-attempts='${API_RETRY_ATTEMPTS}'"
      shift
      shift
      ;;
    --containerd-config-file)
      CONTAINERD_CONFIG_FILE=$2
      log "INFO: --containerd-config-file='${CONTAINERD_CONFIG_FILE}'"
      shift
      shift
      ;;
    --pause-container-account)
      PAUSE_CONTAINER_ACCOUNT=$2
      log "INFO: --pause-container-account='${PAUSE_CONTAINER_ACCOUNT}'"
      shift
      shift
      ;;
    --pause-container-version)
      PAUSE_CONTAINER_VERSION=$2
      log "INFO: --pause-container-version='${PAUSE_CONTAINER_VERSION}'"
      shift
      shift
      ;;
    --dns-cluster-ip)
      DNS_CLUSTER_IP=$2
      log "INFO: --dns-cluster-ip='${DNS_CLUSTER_IP}'"
      shift
      shift
      ;;
    --ip-family)
      IP_FAMILY=$2
      log "INFO: --ip-family='${IP_FAMILY}'"
      shift
      shift
      ;;
    --service-ipv6-cidr)
      SERVICE_IPV6_CIDR=$2
      log "INFO: --service-ipv6-cidr='${SERVICE_IPV6_CIDR}'"
      shift
      shift
      ;;
    --enable-local-outpost)
      ENABLE_LOCAL_OUTPOST=$2
      log "INFO: --enable-local-outpost='${ENABLE_LOCAL_OUTPOST}'"
      shift
      shift
      ;;
    --cluster-id)
      CLUSTER_ID=$2
      log "INFO: --cluster-id='${CLUSTER_ID}'"
      shift
      shift
      ;;
    --mount-bpf-fs)
      MOUNT_BPF_FS=$2
      log "INFO: --mount-bpf-fs='${MOUNT_BPF_FS}'"
      shift
      shift
      ;;
    --local-disks)
      LOCAL_DISKS=$2
      log "INFO: --local-disks='${LOCAL_DISKS}'"
      shift
      shift
      ;;
    *)                   # unknown option
      POSITIONAL+=("$1") # save it in an array for later
      shift              # past argument
      ;;
  esac
done


if [[ ! -z ${LOCAL_DISKS} ]]; then
  setup-local-disks "${LOCAL_DISKS}"
fi

AWS_DEFAULT_REGION=$(imds 'latest/dynamic/instance-identity/document' | jq .region -r)
AWS_SERVICES_DOMAIN=$(imds 'latest/meta-data/services/domain')

ECR_URI=$(/etc/eks/get-ecr-uri.sh "${AWS_DEFAULT_REGION}" "${AWS_SERVICES_DOMAIN}" "${PAUSE_CONTAINER_ACCOUNT:-}")
PAUSE_CONTAINER_IMAGE=${PAUSE_CONTAINER_IMAGE:-$ECR_URI/eks/pause}
PAUSE_CONTAINER="$PAUSE_CONTAINER_IMAGE:$PAUSE_CONTAINER_VERSION"

### kubelet kubeconfig

### To support worker nodes to continue to communicate and connect to local cluster even when the Outpost
### is disconnected from the parent AWS Region, the following specific setup are required:
###    - append entries to /etc/hosts with the mappings of control plane host IP address and API server
###      domain name. So that the domain name can be resolved to IP addresses locally.
###    - use aws-iam-authenticator as bootstrap auth for kubelet TLS bootstrapping which downloads client
###      X.509 certificate and generate kubelet kubeconfig file which uses the client cert. So that the
###      worker node can be authentiacated through X.509 certificate which works for both connected and
####     disconnected state.
if [[ "${ENABLE_LOCAL_OUTPOST}" == "true" ]]; then
  ### append to /etc/hosts file with shuffled mappings of "IP address to API server domain name"
  DOMAIN_NAME=$(echo "$APISERVER_ENDPOINT" | awk -F/ '{print $3}' | awk -F: '{print $1}')
  getent hosts "$DOMAIN_NAME" | shuf >> /etc/hosts

  ### kubelet bootstrap kubeconfig uses aws-iam-authenticator with cluster id to authenticate to cluster
  ###   - if "aws eks describe-cluster" is bypassed, for local outpost, the value of CLUSTER_NAME parameter will be cluster id.
  ###   - otherwise, the cluster id will use the id returned by "aws eks describe-cluster".
  if [[ -z "${CLUSTER_ID}" ]]; then
    log "ERROR: Cluster ID is required when local outpost support is enabled"
    exit 1
  else
    sed -i s,CLUSTER_NAME,$CLUSTER_ID,g /var/lib/kubelet/kubeconfig

    ### use aws-iam-authenticator as bootstrap auth and download X.509 cert used in kubelet kubeconfig
    mv /var/lib/kubelet/kubeconfig /var/lib/kubelet/bootstrap-kubeconfig
    KUBELET_EXTRA_ARGS="--bootstrap-kubeconfig /var/lib/kubelet/bootstrap-kubeconfig $KUBELET_EXTRA_ARGS"
  fi
else
  sed -i s,CLUSTER_NAME,$CLUSTER_NAME,g /var/lib/kubelet/kubeconfig
fi

### kubelet.service configuration

KUBELET_ARGS="--node-ip=$INTERNAL_IP --pod-infra-container-image=$PAUSE_CONTAINER --v=2"

if vercmp "$KUBELET_VERSION" lt "1.26.0"; then
  # TODO: remove this when 1.25 is EOL
  KUBELET_CLOUD_PROVIDER="aws"
else
  KUBELET_CLOUD_PROVIDER="external"
  echo "$(jq ".providerID=\"$(provider-id)\"" $KUBELET_CONFIG)" > $KUBELET_CONFIG
  # When the external cloud provider is used, kubelet will use /etc/hostname as the name of the Node object.
  # If the VPC has a custom `domain-name` in its DHCP options set, and the VPC has `enableDnsHostnames` set to `true`,
  # then /etc/hostname is not the same as EC2's PrivateDnsName.
  # The name of the Node object must be equal to EC2's PrivateDnsName for the aws-iam-authenticator to allow this kubelet to manage it.
  INSTANCE_ID=$(imds /latest/meta-data/instance-id)
  # the AWS CLI currently constructs the wrong endpoint URL on localzones (the availability zone group will be used instead of the parent region)
  # more info: https://github.com/aws/aws-cli/issues/7043
  REGION=$(imds /latest/meta-data/placement/region)
  PRIVATE_DNS_NAME=$(AWS_RETRY_MODE=standard AWS_MAX_ATTEMPTS=10 aws ec2 describe-instances --region $REGION --instance-ids $INSTANCE_ID --query 'Reservations[].Instances[].PrivateDnsName' --output text)
  KUBELET_ARGS="$KUBELET_ARGS --hostname-override=$PRIVATE_DNS_NAME"
fi

KUBELET_ARGS="$KUBELET_ARGS --cloud-provider=$KUBELET_CLOUD_PROVIDER"

mkdir -p /etc/systemd/system

if [[ "$CONTAINER_RUNTIME" = "containerd" ]]; then
  sudo mkdir -p /etc/containerd
  sudo mkdir -p /etc/cni/net.d

  sudo mkdir -p /etc/systemd/system/containerd.service.d
  printf '[Service]\nSlice=runtime.slice\n' | sudo tee /etc/systemd/system/containerd.service.d/00-runtime-slice.conf

  if [[ -n "${CONTAINERD_CONFIG_FILE}" ]]; then
    sudo cp -v "${CONTAINERD_CONFIG_FILE}" /etc/eks/containerd/containerd-config.toml
  fi

  sudo sed -i s,SANDBOX_IMAGE,$PAUSE_CONTAINER,g /etc/eks/containerd/containerd-config.toml

  echo "$(jq '.cgroupDriver="systemd"' "${KUBELET_CONFIG}")" > "${KUBELET_CONFIG}"
  echo "$(jq '.systemReservedCgroup="/system"' "${KUBELET_CONFIG}")" > "${KUBELET_CONFIG}"
  echo "$(jq '.kubeReservedCgroup="/runtime"' "${KUBELET_CONFIG}")" > "${KUBELET_CONFIG}"

  # Check if the containerd config file is the same as the one used in the image build.
  # If different, then restart containerd w/ proper config
  if ! cmp -s /etc/eks/containerd/containerd-config.toml /etc/containerd/config.toml; then
    sudo cp -v /etc/eks/containerd/containerd-config.toml /etc/containerd/config.toml
    sudo cp -v /etc/eks/containerd/sandbox-image.service /etc/systemd/system/sandbox-image.service
    sudo chown root:root /etc/systemd/system/sandbox-image.service
    systemctl daemon-reload
    systemctl enable containerd sandbox-image
    systemctl restart sandbox-image containerd
  fi
  sudo cp -v /etc/eks/containerd/kubelet-containerd.service /etc/systemd/system/kubelet.service
  sudo chown root:root /etc/systemd/system/kubelet.service
  # Validate containerd config
  sudo containerd config dump > /dev/null

  # --container-runtime flag is gone in 1.27+
  # TODO: remove this when 1.26 is EOL
  if vercmp "$KUBELET_VERSION" lt "1.27.0"; then
    KUBELET_ARGS="$KUBELET_ARGS --container-runtime=remote"
  fi
else
  log "ERROR: unsupported container runtime: '${CONTAINER_RUNTIME}'"
  exit 1
fi

mkdir -p /etc/systemd/system/kubelet.service.d

cat << EOF > /etc/systemd/system/kubelet.service.d/10-kubelet-args.conf
[Service]
Environment='KUBELET_ARGS=$KUBELET_ARGS'
EOF

if [[ -n "$KUBELET_EXTRA_ARGS" ]]; then
  cat << EOF > /etc/systemd/system/kubelet.service.d/30-kubelet-extra-args.conf
[Service]
Environment='KUBELET_EXTRA_ARGS=$KUBELET_EXTRA_ARGS'
EOF
fi

systemctl daemon-reload
systemctl enable kubelet
systemctl start kubelet

log "INFO: complete!"

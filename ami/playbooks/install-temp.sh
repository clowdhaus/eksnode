#!/usr/bin/env bash

set -o pipefail
set -o nounset
set -o errexit
IFS=$'\n\t'
export AWS_DEFAULT_OUTPUT="json"

################################################################################
### awscli #####################################################
################################################################################

### isolated regions can't communicate to awscli.amazonaws.com so installing awscli through yum
ISOLATED_REGIONS="${ISOLATED_REGIONS:-us-iso-east-1 us-iso-west-1 us-isob-east-1}"
if ! [[ ${ISOLATED_REGIONS} =~ $BINARY_BUCKET_REGION ]]; then
  # https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html
  echo "Installing awscli v2 bundle"
  AWSCLI_DIR="${WORKING_DIR}/awscli-install"
  mkdir "${AWSCLI_DIR}"
  curl \
    --silent \
    --show-error \
    --retry 10 \
    --retry-delay 1 \
    -L "https://awscli.amazonaws.com/awscli-exe-linux-${MACHINE}.zip" -o "${AWSCLI_DIR}/awscliv2.zip"
  unzip -q "${AWSCLI_DIR}/awscliv2.zip" -d ${AWSCLI_DIR}
  sudo "${AWSCLI_DIR}/aws/install" --bin-dir /bin/ --update
else
  echo "Installing awscli package"
  sudo yum install -y awscli
fi

###############################################################################
### Containerd setup ##########################################################
###############################################################################

# Handle at bootstrap - dynamic values
if [ -f "/etc/eks/containerd/containerd-config.toml" ]; then
  ## this means we are building a gpu ami and have already placed a containerd configuration file in /etc/eks
  echo "containerd config is already present"
else
  sudo mv $WORKING_DIR/containerd-config.toml /etc/eks/containerd/containerd-config.toml
fi
# Handle at bootstrap - dynamic values
sudo mv $WORKING_DIR/kubelet-containerd.service /etc/eks/containerd/kubelet-containerd.service
# TODO - handle inside eksnode CLI
sudo mv $WORKING_DIR/pull-sandbox-image.sh /etc/eks/containerd/pull-sandbox-image.sh
sudo mv $WORKING_DIR/pull-image.sh /etc/eks/containerd/pull-image.sh
sudo chmod +x /etc/eks/containerd/pull-sandbox-image.sh
sudo chmod +x /etc/eks/containerd/pull-image.sh


cat << EOF | sudo tee -a /etc/modules-load.d/containerd.conf
overlay
br_netfilter
EOF

cat << EOF | sudo tee -a /etc/sysctl.d/99-kubernetes-cri.conf
net.bridge.bridge-nf-call-ip6tables = 1
net.bridge.bridge-nf-call-iptables = 1
net.ipv4.ip_forward = 1
EOF

################################################################################
### Logrotate ##################################################################
################################################################################

# kubelet uses journald which has built-in rotation and capped size.
# See man 5 journald.conf
sudo mv $WORKING_DIR/logrotate-kube-proxy /etc/logrotate.d/kube-proxy
sudo mv $WORKING_DIR/logrotate.conf /etc/logrotate.conf
sudo chown root:root /etc/logrotate.d/kube-proxy
sudo chown root:root /etc/logrotate.conf
sudo mkdir -p /var/log/journal

################################################################################
### Kubernetes #################################################################
################################################################################

sudo mkdir -p /etc/kubernetes/manifests
sudo mkdir -p /var/lib/kubernetes
sudo mkdir -p /var/lib/kubelet
sudo mkdir -p /opt/cni/bin

echo "Downloading binaries from: s3://$BINARY_BUCKET_NAME"
S3_DOMAIN="amazonaws.com"
if [ "$BINARY_BUCKET_REGION" = "cn-north-1" ] || [ "$BINARY_BUCKET_REGION" = "cn-northwest-1" ]; then
  S3_DOMAIN="amazonaws.com.cn"
elif [ "$BINARY_BUCKET_REGION" = "us-iso-east-1" ] || [ "$BINARY_BUCKET_REGION" = "us-iso-west-1" ]; then
  S3_DOMAIN="c2s.ic.gov"
elif [ "$BINARY_BUCKET_REGION" = "us-isob-east-1" ]; then
  S3_DOMAIN="sc2s.sgov.gov"
fi
S3_URL_BASE="https://$BINARY_BUCKET_NAME.s3.$BINARY_BUCKET_REGION.$S3_DOMAIN/$KUBERNETES_VERSION/$KUBERNETES_BUILD_DATE/bin/linux/$ARCH"
S3_PATH="s3://$BINARY_BUCKET_NAME/$KUBERNETES_VERSION/$KUBERNETES_BUILD_DATE/bin/linux/$ARCH"

BINARIES=(
  kubelet
  aws-iam-authenticator
)
for binary in ${BINARIES[*]}; do
  if [[ -n "$AWS_ACCESS_KEY_ID" ]]; then
    echo "AWS cli present - using it to copy binaries from s3."
    aws s3 cp --region $BINARY_BUCKET_REGION $S3_PATH/$binary .
    aws s3 cp --region $BINARY_BUCKET_REGION $S3_PATH/$binary.sha256 .
  else
    echo "AWS cli missing - using wget to fetch binaries from s3. Note: This won't work for private bucket."
    sudo wget $S3_URL_BASE/$binary
    sudo wget $S3_URL_BASE/$binary.sha256
  fi
  sudo sha256sum -c $binary.sha256
  sudo chmod +x $binary
  sudo mv $binary /usr/bin/
done

# Verify that the aws-iam-authenticator is at last v0.5.9 or greater. Otherwise, nodes will be
# unable to join clusters due to upgrading to client.authentication.k8s.io/v1beta1
iam_auth_version=$(sudo /usr/bin/aws-iam-authenticator version | jq -r .Version)
if vercmp "$iam_auth_version" lt "v0.5.9"; then
  # To resolve this issue, you need to update the aws-iam-authenticator binary. Using binaries distributed by EKS
  # with kubernetes_build_date 2022-10-31 or later include v0.5.10 or greater.
  echo "❌ The aws-iam-authenticator should be on version v0.5.9 or later. Found $iam_auth_version"
  exit 1
fi

# Handle at bootstrap - dynamic values
sudo mv $WORKING_DIR/kubelet-kubeconfig /var/lib/kubelet/kubeconfig
sudo chown root:root /var/lib/kubelet/kubeconfig
# Handle at bootstrap - dynamic values
sudo mv $WORKING_DIR/kubelet.service /etc/systemd/system/kubelet.service
sudo chown root:root /etc/systemd/system/kubelet.service
# Handle at bootstrap - dynamic values
sudo mv $WORKING_DIR/kubelet-config.json /etc/kubernetes/kubelet/kubelet-config.json
sudo chown root:root /etc/kubernetes/kubelet/kubelet-config.json

sudo systemctl daemon-reload
# Disable the kubelet until the proper dropins have been configured
sudo systemctl disable kubelet

################################################################################
### EKS ########################################################################
################################################################################


sudo mv $WORKING_DIR/eni-max-pods.txt /etc/eks/eni-max-pods.txt
# TODO - create psuedo file for backwards compat?
sudo mv $WORKING_DIR/bootstrap.sh /etc/eks/bootstrap.sh
sudo chmod +x /etc/eks/bootstrap.sh
# TODO - create psuedo file for backwards compat?
sudo mv $WORKING_DIR/max-pods-calculator.sh /etc/eks/max-pods-calculator.sh
sudo chmod +x /etc/eks/max-pods-calculator.sh

################################################################################
### ECR CREDENTIAL PROVIDER ####################################################
################################################################################
ECR_CREDENTIAL_PROVIDER_BINARY="ecr-credential-provider"
if [[ -n "$AWS_ACCESS_KEY_ID" ]]; then
  echo "AWS cli present - using it to copy ${ECR_CREDENTIAL_PROVIDER_BINARY} from s3."
  aws s3 cp --region $BINARY_BUCKET_REGION $S3_PATH/$ECR_CREDENTIAL_PROVIDER_BINARY .
else
  echo "AWS cli missing - using wget to fetch ${ECR_CREDENTIAL_PROVIDER_BINARY} from s3. Note: This won't work for private bucket."
  sudo wget "$S3_URL_BASE/$ECR_CREDENTIAL_PROVIDER_BINARY"
fi
sudo chmod +x $ECR_CREDENTIAL_PROVIDER_BINARY
sudo mkdir -p /etc/eks/image-credential-provider
sudo mv $ECR_CREDENTIAL_PROVIDER_BINARY /etc/eks/image-credential-provider/

# Handle at bootstrap - dynamic values
sudo mv $WORKING_DIR/ecr-credential-provider-config.json /etc/eks/image-credential-provider/config.json

################################################################################
### Cache Images ###############################################################
################################################################################

if [[ "$CACHE_CONTAINER_IMAGES" == "true" ]] && ! [[ ${ISOLATED_REGIONS} =~ $BINARY_BUCKET_REGION ]]; then
  AWS_DOMAIN=$(imds 'latest/meta-data/services/domain')
  ECR_URI=$(/etc/eks/get-ecr-uri.sh "${BINARY_BUCKET_REGION}" "${AWS_DOMAIN}")

  PAUSE_CONTAINER="${ECR_URI}/eks/pause:${PAUSE_CONTAINER_VERSION}"
  cat /etc/eks/containerd/containerd-config.toml | sed s,SANDBOX_IMAGE,$PAUSE_CONTAINER,g | sudo tee /etc/eks/containerd/containerd-cached-pause-config.toml
  sudo cp -v /etc/eks/containerd/containerd-cached-pause-config.toml /etc/containerd/config.toml
  sudo cp -v /etc/eks/containerd/sandbox-image.service /etc/systemd/system/sandbox-image.service
  sudo chown root:root /etc/systemd/system/sandbox-image.service
  sudo systemctl daemon-reload
  sudo systemctl start containerd
  sudo systemctl enable containerd sandbox-image

  K8S_MINOR_VERSION=$(echo "${KUBERNETES_VERSION}" | cut -d'.' -f1-2)

  #### Cache kube-proxy images starting with the addon default version and the latest version
  KUBE_PROXY_ADDON_VERSIONS=$(aws eks describe-addon-versions --addon-name kube-proxy --kubernetes-version=${K8S_MINOR_VERSION})
  KUBE_PROXY_IMGS=()
  if [[ $(jq '.addons | length' <<< $KUBE_PROXY_ADDON_VERSIONS) -gt 0 ]]; then
    DEFAULT_KUBE_PROXY_FULL_VERSION=$(echo "${KUBE_PROXY_ADDON_VERSIONS}" | jq -r '.addons[] .addonVersions[] | select(.compatibilities[] .defaultVersion==true).addonVersion')
    DEFAULT_KUBE_PROXY_VERSION=$(echo "${DEFAULT_KUBE_PROXY_FULL_VERSION}" | cut -d"-" -f1)
    DEFAULT_KUBE_PROXY_PLATFORM_VERSION=$(echo "${DEFAULT_KUBE_PROXY_FULL_VERSION}" | cut -d"-" -f2)

    LATEST_KUBE_PROXY_FULL_VERSION=$(echo "${KUBE_PROXY_ADDON_VERSIONS}" | jq -r '.addons[] .addonVersions[] .addonVersion' | sort -V | tail -n1)
    LATEST_KUBE_PROXY_VERSION=$(echo "${LATEST_KUBE_PROXY_FULL_VERSION}" | cut -d"-" -f1)
    LATEST_KUBE_PROXY_PLATFORM_VERSION=$(echo "${LATEST_KUBE_PROXY_FULL_VERSION}" | cut -d"-" -f2)

    KUBE_PROXY_IMGS=(
      ## Default kube-proxy images
      "${ECR_URI}/eks/kube-proxy:${DEFAULT_KUBE_PROXY_VERSION}-${DEFAULT_KUBE_PROXY_PLATFORM_VERSION}"
      "${ECR_URI}/eks/kube-proxy:${DEFAULT_KUBE_PROXY_VERSION}-minimal-${DEFAULT_KUBE_PROXY_PLATFORM_VERSION}"

      ## Latest kube-proxy images
      "${ECR_URI}/eks/kube-proxy:${LATEST_KUBE_PROXY_VERSION}-${LATEST_KUBE_PROXY_PLATFORM_VERSION}"
      "${ECR_URI}/eks/kube-proxy:${LATEST_KUBE_PROXY_VERSION}-minimal-${LATEST_KUBE_PROXY_PLATFORM_VERSION}"
    )
  fi

  #### Cache VPC CNI images starting with the addon default version and the latest version
  VPC_CNI_ADDON_VERSIONS=$(aws eks describe-addon-versions --addon-name vpc-cni --kubernetes-version=${K8S_MINOR_VERSION})
  VPC_CNI_IMGS=()
  if [[ $(jq '.addons | length' <<< $VPC_CNI_ADDON_VERSIONS) -gt 0 ]]; then
    DEFAULT_VPC_CNI_VERSION=$(echo "${VPC_CNI_ADDON_VERSIONS}" | jq -r '.addons[] .addonVersions[] | select(.compatibilities[] .defaultVersion==true).addonVersion')
    LATEST_VPC_CNI_VERSION=$(echo "${VPC_CNI_ADDON_VERSIONS}" | jq -r '.addons[] .addonVersions[] .addonVersion' | sort -V | tail -n1)
    CNI_IMG="${ECR_URI}/amazon-k8s-cni"
    CNI_INIT_IMG="${CNI_IMG}-init"

    VPC_CNI_IMGS=(
      ## Default VPC CNI Images
      "${CNI_IMG}:${DEFAULT_VPC_CNI_VERSION}"
      "${CNI_INIT_IMG}:${DEFAULT_VPC_CNI_VERSION}"

      ## Latest VPC CNI Images
      "${CNI_IMG}:${LATEST_VPC_CNI_VERSION}"
      "${CNI_INIT_IMG}:${LATEST_VPC_CNI_VERSION}"
    )
  fi

  CACHE_IMGS=(
    "${PAUSE_CONTAINER}"
    ${KUBE_PROXY_IMGS[@]+"${KUBE_PROXY_IMGS[@]}"}
    ${VPC_CNI_IMGS[@]+"${VPC_CNI_IMGS[@]}"}
  )
  PULLED_IMGS=()

  for img in "${CACHE_IMGS[@]}"; do
    ## only kube-proxy-minimal is vended for K8s 1.24+
    if [[ "${img}" == *"kube-proxy:"* ]] && [[ "${img}" != *"-minimal-"* ]] && vercmp "${K8S_MINOR_VERSION}" gteq "1.24"; then
      continue
    fi
    ## Since eksbuild.x version may not match the image tag, we need to decrement the eksbuild version until we find the latest image tag within the app semver
    eksbuild_version="1"
    if [[ ${img} == *'eksbuild.'* ]]; then
      eksbuild_version=$(echo "${img}" | grep -o 'eksbuild\.[0-9]\+' | cut -d'.' -f2)
    fi
    ## iterate through decrementing the build version each time
    for build_version in $(seq "${eksbuild_version}" -1 1); do
      img=$(echo "${img}" | sed -E "s/eksbuild.[0-9]+/eksbuild.${build_version}/")
      if /etc/eks/containerd/pull-image.sh "${img}"; then
        PULLED_IMGS+=("${img}")
        break
      elif [[ "${build_version}" -eq 1 ]]; then
        exit 1
      fi
    done
  done

  #### Tag the pulled down image for all other regions in the partition
  for region in $(aws ec2 describe-regions --all-regions | jq -r '.Regions[] .RegionName'); do
    for img in "${PULLED_IMGS[@]}"; do
      regional_img="${img/$BINARY_BUCKET_REGION/$region}"
      sudo ctr -n k8s.io image tag "${img}" "${regional_img}" || :
      ## Tag ECR fips endpoint for supported regions
      if [[ "${region}" =~ (us-east-1|us-east-2|us-west-1|us-west-2|us-gov-east-1|us-gov-east-2) ]]; then
        regional_fips_img="${regional_img/.ecr./.ecr-fips.}"
        sudo ctr -n k8s.io image tag "${img}" "${regional_fips_img}" || :
        sudo ctr -n k8s.io image tag "${img}" "${regional_fips_img/-eksbuild.1/}" || :
      fi
      ## Cache the non-addon VPC CNI images since "v*.*.*-eksbuild.1" is equivalent to leaving off the eksbuild suffix
      if [[ "${img}" == *"-cni"*"-eksbuild.1" ]]; then
        sudo ctr -n k8s.io image tag "${img}" "${regional_img/-eksbuild.1/}" || :
      fi
    done
  done
fi

################################################################################
### SSM Agent ##################################################################
################################################################################

sudo yum install -y amazon-ssm-agent

################################################################################
### AMI Metadata ###############################################################
################################################################################

BASE_AMI_ID=$(imds /latest/meta-data/ami-id)
cat << EOF > "${WORKING_DIR}/release"
BASE_AMI_ID="$BASE_AMI_ID"
BUILD_TIME="$(date)"
BUILD_KERNEL="$(uname -r)"
ARCH="$(uname -m)"
EOF
sudo mv "${WORKING_DIR}/release" /etc/eks/release
sudo chown -R root:root /etc/eks

################################################################################
### Stuff required by "protectKernelDefaults=true" #############################
################################################################################

cat << EOF | sudo tee -a /etc/sysctl.d/99-amazon.conf
vm.overcommit_memory=1
kernel.panic=10
kernel.panic_on_oops=1
EOF

################################################################################
### Setting up sysctl properties ###############################################
################################################################################

echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf
echo fs.inotify.max_user_instances=8192 | sudo tee -a /etc/sysctl.conf
echo vm.max_map_count=524288 | sudo tee -a /etc/sysctl.conf

################################################################################
### adding log-collector-script ################################################
################################################################################
sudo mkdir -p /etc/eks/log-collector-script/
sudo cp $WORKING_DIR/log-collector-script/eks-log-collector.sh /etc/eks/log-collector-script/

################################################################################
### Remove Yum Update from cloud-init config ###################################
################################################################################
sudo sed -i \
  's/ - package-update-upgrade-install/# Removed so that nodes do not have version skew based on when the node was started.\n# - package-update-upgrade-install/' \
  /etc/cloud/cloud.cfg
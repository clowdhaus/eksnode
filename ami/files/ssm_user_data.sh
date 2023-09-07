#!/usr/bin/sh

# Amazon Linux 2023 Minimal does not come with the SSM agent installed by default
dnf install -y https://s3.us-east-1.amazonaws.com/amazon-ssm-us-east-1/latest/linux_amd64/amazon-ssm-agent.rpm

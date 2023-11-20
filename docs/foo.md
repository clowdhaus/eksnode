## High Level Process

There are primarily two phases - image creation and node provisioning. The image creation process is responsible for installing components and setting static configuration while the node provisioning process is responsible for performing dynamic configuration that cannot, or should not, be baked into the image. There is a tradeoff between image creation and node provisioning with respect to image flexibility and size versus the time required to join the cluster.

The more that can be baked into the image, the less time that is required for the node to join the cluster. However, this also means that the image will be either less flexible with respect to the various configurations it supports or the image will increase in size resulting in slower startup times. Conversely, making the image more flexible by moving more tasks to the node provisioning process will result in more time required for the node to join the cluster.

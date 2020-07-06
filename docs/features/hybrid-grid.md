# Hybrid grid

The [Getting Started Guide](../getting-started.md) covers basic setups in either Kubernetes or Docker. However, in certain scenarios it might be required to include other devices that can't be enslaved to a cluster.

Imagine a software testing use-case where you have the following setup:

* Chrome & Firefox pods running in K8s
* Safari on external Mac Mini
* Edge on external Windows PC

All these devices can be collated behind a single grid endpoint. In theory, every component of the grid can be hosted in and scaled to any device regardless of whether or not it is in the cluster or not. However, for larger instances it is recommended to run the central grid components in Kubernetes and add external devices to extend its capabilities with e.g. Safari Browsers.

## Requirements

In order to add external devices a few requirements have to be met:

1. Proxys and Manager pods have to be able to reach the device
2. Redis database has to be accessible by the device

Once these prerequisites are met, continue to the next sections.

## Local orchestrator

The orchestrator service is responsible for scheduling resources like Kubernetes pods or Docker containers which then in turn run the browsers. To use a local browser like Safari a single-instance orchestrator is required.

!!! todo
    This feature is on the horizon.
    
    Even though the implementation has not yet been started, it has been incorporated during the design stage and its complexity is rather mild.
    
    If you need this feature, please open up an Issue or +1 an existing one regarding this feature.

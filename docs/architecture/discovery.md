# Service discovery

This project uses multiple [different services](./services.md) to accomplish a variety of tasks. Internal communication is handled through the Pub/Sub channel principle. However, larger payloads which are user-centric (i.e. caused by the user and returned to the user without modification) are transmitted over HTTP. This inherently requires a direct connection between the participating services. For this reason, a service discovery mechanism has been introduced.

This mechanism generally operates in a semi-active manner but includes some additions to improve efficiency. The details are listed below

## Active discovery

When a service wants to discover another service, it follows these steps:

1. Build a `ServiceDescriptor` of the service to discover
2. Derive a discovery messaging channel
3. Send a request
4. Listen on the general discovery response channel (non-specific to any service)
5. Use any replies to establish a connection

Note that requests are made to a specific channel where the channel address indicates which service to discovery. The reply, however, will be sent to a general channel. This allows for passive discovery, as explained below.

## Passive discovery

Independent of the request-response mechanism, each service passively listens on the response channel. This allows capturing of responses to requests sent by other services. These responses will subsequently be stored in a local LRU cache. Especially for the proxy service, this increases performance (behind a load-balancer the load is randomly distributed to all proxies and if one user accesses some session it is likely that he will send further requests to that session in the future).

## Preemptive caching

In addition to passive discovery, preemptive cache filling is done. On startup, a service broadcasts its endpoint to the discovery response channel. This fills the caches of other services thanks to passive discovery and reduces the number of round-trips required to zero (at the cost of some memory).

## Cache poisoning

The previously mentioned cache can hold multiple endpoints for a given `ServiceDescriptor`. However, over time some of these endpoints may become unavailable. If such an endpoint is encountered, the corresponding cache entry will be purged and either one of the remaining cache entries will be tried or a new active discovery is started. This process is repeated for a limited number of times.

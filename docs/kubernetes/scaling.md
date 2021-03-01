# Scaling

As the grid is optimized for performance, it should work for most small-scale scenarios. However, if you are running into performance issues it can be scaled up to meet demand. The grid is split into multiple distinct components and each can be replicated individually (apart from the database — but it should outperform everything else [by a long stretch](https://redis.io/topics/benchmarks)).

You should look at which component requires scaling. Below is a list of common scenarios where bottlenecks are expected.

## Concurrent sessions

By default, the number of concurrent sessions is limited to five per orchestrator and one Kubernetes orchestrator. Normally, only one orchestrator is required even for very large setups so the per-orchestrator limit should be used.

To change the number of concurrent sessions allowed merge and apply the following helm value as described [here](./configuration.md#changing-the-defaults):

```yaml
maxSessionsPerOrchestrator: 5
```

!!! note
    Make sure that your K8s service account has a sufficient quota available to create the required number of pods for the sessions.

## Traffic congestion

Another common bottleneck, which is especially common with regular Selenium Grids, is the proxy server. Due to protocol constraints all traffic has to be routed through an intermediate instance, which inherently creates a choke point. This can be remedied by the microservice architecture of WebGrid.

To increase the number of proxy servers that route session control traffic, merge and apply the following helm value as described [here](./configuration.md#changing-the-defaults):

```yaml
replicaCount:
  proxy: 2
```
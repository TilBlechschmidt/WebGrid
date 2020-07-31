# Error handling
In an ideal world, individual services are unable to fail by themselves. A service may reject a request as unfulfillable but always keeps processing. However due to the volatile nature of external resources like databases or cluster provisioners a monitoring system is required to handle outages.

In this case a job system has been implemented where each job can have a dependency on an external resource. If this external resource dies, all dependent jobs will be cancelled. All root jobs of a component may then be restarted once the required resources become available again.

Below is a rough outline of the architectural concepts involved.

## Parameter
Parameters are constant values set at runtime, usually through the environment or command-line arguments.

## Resource
Resources represent external data handlers like Redis, K8s or Storage. There are two types of resources, stateful and stateless. Stateful resources may report their current availability status while stateless ones are always reported as available.

Resources have associated structures called Providers. A provider is responsible for creating handles to a resource. Each requested handle must have an associated ID. These IDs may not be unique if the underlying connection is shared. Handle IDs can be used for dependency tracking and job restarts in case a handle dies.

Resources may have service-specific initialization jobs that are executed once they become available. These can be set on the corresponding resource provider upon creation.

## Contract
Contracts atomically define behavior through a set of given input resource states and expected outputs. Contracts use placeholders for actual values much like class definitions.

### Request
A specific instance of a contract with bound values is called a request.

## Job
Jobs serve to fulfill one contract and may repeatedly do so. A job is considered *healthy* if it is able to fulfill its assigned contract.

### Task
Jobs may have children called Tasks that process a single instance of a contract. For example the job could be an HTTP server which schedules new tasks for each incoming request. These children may then be terminated by the service if their resource handles go stale and the job may then take corresponding actions e.g. send an error code.

## Services
Services are units that manage a set of related jobs and hold the required resource providers which can be configured through parameters. They are responsible for starting jobs and terminating those whose resources became stale. Jobs are restarted if the required resources are available again. Each job receives a handle to spawn ephemeral tasks which are not respawned. Additionally a future is passed that signals a clean shutdown condition — for proper SIGTERM handling.
# Port assignments
The following ports are used internally for communication between various components. The base port has been set to 40001 which is unassigned according to the current IANA port assignment list. The ports below are either offsets from the base port or absolute numbers.

## Manager
- -> Redis (6397)
- -> Node (+2)
- <- HTTP (+0)

## Metrics
- -> Redis (6397)
- <- HTTP (+1)

## Node
- -> Redis (6397)
- <- HTTP (+2)
- -- Driver HTTP (Base Image environment)

## Orchestrator
- -> Redis (6397)
- <- HTTP (+3)

## Proxy
- -> Redis (6397)
- -> Manager (+0)
- -> Node (+2)
- <- HTTP (+4)
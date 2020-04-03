# Data store

## Root lists
```
orchestrators = Set<string> (uuids)
managers = Set<string> (uuids)
sessions.active = Set<string> (uuids)
sessions.terminated = Set<string> (uuids)
```

## Configuration
```
timeouts = Hashes {
	queue = number (seconds)
	scheduling = number (seconds)
	nodeStartup = number (seconds)
	driverStartup = number (seconds)
	sessionTermination = number (seconds)
	slotReclaimInterval = number (seconds)
}
```

## Sessions
```
session:<ID>:heartbeat.node = number EX 60s
session:<ID>:heartbeat.manager = number EX 30s

session:<ID>:slot = string (slot ID)
// Orchestrator being a list is an implementation detail.
session:<ID>:orchestrator = List<string> (orchestrator ID)

session:<ID>:log = Stream {
	component = 'node' | 'orchestrator' | 'manager' | 'proxy'
	level = 'info' | 'warn' | 'fail'
	code = string (event type, see below)
	meta = string (additional information)
}

session:<ID>:status = Hashes {
	queuedAt = string (RFC 3339)
	pendingAt = string (RFC 3339)
	aliveAt = string (RFC 3339)
	terminatedAt = string (RFC 3339)
}

session:<ID>:capabilities = Hashes {
	requested = string (JSON)
	actual = string (JSON)
}

session:<ID>:upstream = Hashes {
	host = string
	port = number
	driverSessionID = string
}

session:<ID>:downstream = Hashes {
	host = string
	userAgent = string
	lastSeen = string (RFC 3339)
}

session:<ID>:statistics = Hashes {
	fulfilledRequests = number
}
```

### Log event codes
Warnings are informative but automatic recovery is possible while failures are fatal and the session terminates.

#### Node
- Info
	- `BOOT` node has become active
	- `DSTART` driver in startup
	- `DALIVE` driver has become responsive
	- `LSINIT` local session created
	- `CLOSED` session closed by downstream client
	- `HALT` node enters shutdown
- Fail
	- `DTIMEOUT` driver has not become responsive
	- `DFAILURE` driver process reported an error
		- TODO: Throw this if the driver crashes.
	- `STIMEOUT` session has been inactive too long
	- `TERM` node terminates due to fault condition

#### Orchestrator
- Info
	- `SCHED` node is being scheduled for startup
- Fail
	- `STARTFAIL` creation/startup failure

#### Manager
- Info
	- `QUEUED` session has been queued at orchestrators
	- `NALLOC` node slot has been allocated
	- `PENDING` awaiting node startup
	- `NALIVE` node has become responsive, client served
- Warn
	- `CLEFT` client left before scheduling completed
- Fail
	- `QUNAVAILABLE` no orchestrator can satisfy the capabilities
	- `QTIMEOUT` timed out waiting in queue
	- `OTIMEOUT` timed out waiting for orchestrator to schedule node
	- `NTIMEOUT` timed out waiting for node to become responsive

## Orchestrators
```
orchestrator:<ID> = Hashes {
	type = 'local' | 'docker' | 'k8s'
}

// TODO Add log for reclaiming, scheduling and scaling

orchestrator:<ID>:heartbeat = number EX 60
orchestrator:<ID>:capabilities = Set<string> (JSONs)

orchestrator:<ID>:slots.reclaimed = List<string> (slot ID)
orchestrator:<ID>:slots.available = List<string> (slot ID)
orchestrator:<ID>:slots = Set<string> (slot ID)

orchestrator:<ID>:backlog = List<string> (session ID)
orchestrator:<ID>:pending = List<string> (session ID)
```

[Reliable queue documentation](https://redis.io/commands/rpoplpush#pattern-reliable-queue)

## Manager
```
manager:<ID>:heartbeat = number EX 120

manager:<ID> = Hashes {
	host = string
	port = number
}
```
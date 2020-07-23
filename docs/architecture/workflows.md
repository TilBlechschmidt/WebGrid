# Workflows

Below is a list of common process workflows that happen within WebGrid. They may help understanding how the grid operates internally.

## Scheduling workflow
0. Orchestrator puts available slots into `:slots.available` on startup by running a reclaim cycle
1. Manager receives client request, runs [session creation workflow](#session-creation-workflow)
2. Manager pulls slot from orchestrators into `session:<ID>:slot`
	- This operation has to be implemented using `BLPOP` and thus might crash between the `BLPOP` and `SET` operations, making the slot vanish. If this happens the orchestrators reclaim cycle may make this slot available again (more on that later)
3. Manager pushes the sessionID into the orchestrators `:backlog`
4. Orchestrator sequentially processes the backlog, moving messages into pending while provisioning the nodes
5. Orchestrator notifies the manager
	- Removes the sessionID from pending
	- Sets the sessions `:status:pendingAt`
	- Pushes its ID into the sessions `:orchestrator` key
6. Manager watches the sessions `:orchestrator` key using `BRPOPLPUSH` for it to become available
7. Manager runs health-check against node (http://node/status)
8. Manager replies to client
9. Manager sets `:status:aliveAt` property, effectively moving slot responsibility to the node from now on
10. Node runs [session termination workflow](#session-termination-workflow) when client ends session

If the orchestrator finds any sessions in its pending list on startup, it may fulfil these requests **if** the referenced session has not entered a terminated state (the manager ran into a timeout waiting for the orchestrator). Otherwise it may delete the task.

## Slot reclaim workflow

Since any given component in the system may fail during slot interaction the orchestrator has to reclaim slots that have become stale/unused but are not properly returned. In order to do so it follows these steps, keeping an internal list of reclaimed slot-IDs:

- Iterate sessions that have not entered a terminated state and whose slot is owned by this orchestrator
	- If `:status:aliveAt` is not set and there is no `:heartbeat.manager`, run [session termination workflow](#session-termination-workflow)
	- Else if there is no `:heartbeat.node`, run [session termination workflow](#session-termination-workflow)
	- Else the slot is still in use
- If not all slots are either reclaimed or in use, the missing slots may be added back to the list of available slots

This reclaim cycle may be used as a cleanup cycle by the orchestrator to terminate any orphaned nodes that are not referenced by an alive session in the database.

### Changing the number of slots for an orchestrator
This is done by the orchestrator on startup (target value is read from its configuration/environment).

#### Adding new slots
The number of slots may be increased by simply adding slots to `orchestrator:<ID>:slots` and `orchestrator:<ID>:slots.available`

#### Removing active slots
To decrease the slot amount, queue a number of `BRPOP` statements on the `:slots.available` and remove the returned slots from the `:slots` list

## Session creation workflow
1. Generate ID
2. Write initial information
	- `session:<ID>:status:queuedAt`
	- `session:<ID>:capabilities:requested`
	- `session:<ID>:downstream:*`
5. Append sessionID to `sessions.active`

## Session termination workflow
1. Move sessions `:slot` back into orchestrators `:slots.available`
2. Move sessions ID from `sessions.active` to `sessions.terminated`
3. Set sessions `:status:terminatedAt` to the current time
4. Delete `:heartbeat.node`
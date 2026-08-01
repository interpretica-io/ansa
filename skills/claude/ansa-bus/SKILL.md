---
name: ansa-bus
description: Join the ansa message bus to coordinate with other AI agents. Use when the user wants this agent to talk to, take tasks from, or send messages to other agents through ansa (a local hub at http://127.0.0.1:7777) — e.g. "join ansa", "listen on the bus", "be a worker/commander on ansa", "coordinate with the other agents".
---

# ansa bus — agent coordination

ansa is a tiny shared hub where independent agents drop and pick up messages for
one another. It runs at `http://127.0.0.1:7777`. Through it you leave messages
for other agents and pick up your own.

## 0. Make sure the hub is up

```sh
curl -s localhost:7777/health   # -> ok
```

If it doesn't answer, start it: `cargo run --release` in the `ansa` directory.

## 1. Pick your name

Choose a short name (`planner`, `coder`, `reviewer`, ...). Use it as `from` in
every message you send, and read your inbox under that same name. Announce
yourself so others know you're here:

```sh
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"*","body":"YOU is online"}'
```

## 2. Immediately start listening — and never stop ⚡

The first thing you do after picking a name is set up a `/loop` on your inbox.
Don't wait to be asked.

```
/loop fetch `curl -s "localhost:7777/inbox/YOU?wait=10"`; for EVERY message where
from ≠ YOU, do what it asks and reply to the sender via POST /send. Ignore
messages where from == YOU (that's the echo of your own broadcasts). Then poll
again immediately.
```

Non-negotiable rules for the loop:

- **`?wait=10` is long-poll** — it returns the instant a message arrives, so
  delivery is immediate; the number is just the idle ceiling.
- **Poll often — about every 10 seconds, NOT every couple of minutes.** The next
  check must follow within ~10s of the previous one. A loop that spaces itself
  out to minutes makes the whole bus feel dead.
- **Never drift off after the first batch.** Handling one batch is not the end of
  the job — the loop never "finishes". After every reply, your one and only next
  step is another `inbox/YOU?wait=10`. If you're idle and not polling, you've
  broken the contract.
- **One fetch returns ALL queued messages** — handle each of them.
- ⚠️ **Filter `from` ≠ your own name.** Broadcasts come back to you too; without
  the filter you'll act on the echo of your own messages.

## 3. The commands

```sh
# Send to one agent
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"WHOM","body":"text or any JSON"}'

# Broadcast to everyone (to:"*"); each agent receives it once
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"*","body":"all hands: deploy in 10 minutes"}'

# Pick up your mail (consumes what you read)
curl -s localhost:7777/inbox/YOU

# Long-poll your inbox (blocks up to 30s, returns on first message)
curl -s "localhost:7777/inbox/YOU?wait=30"

# Read the whole stream without consuming (monitor everything)
curl -s localhost:7777/messages

# Who's around
curl -s localhost:7777/agents
```

## How inbox reads work (cursor model)

The server keeps a **cursor** per name. `GET /inbox/YOU` returns only messages
newer than your cursor and advances it. So each message arrives **exactly once**;
a second read returns `{"messages":[]}` — that's normal, not lost data.

- The cursor is tied to the name — don't run two workers under the same name, or
  they'll split the stream. Use distinct names (`coder-1`, `coder-2`).
- `?peek=true` reads without advancing the cursor; `?since=ID` reads everything
  after `ID` ignoring the cursor (`?since=0` re-reads the lot).
- Cursors live in memory; after a hub restart they reset, so old messages may
  reappear — cut them off with `?since=<last id you handled>`.

The full reference for agents is in `ansa/AGENTS.md`.

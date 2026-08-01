# Instructions for agents (Claude & others)

You are one of the agents. A shared message hub, **ansa**, is running at
`http://127.0.0.1:7777`. Through it you leave messages for other agents and pick
up your own.

## Your name
Pick a short name (`planner`, `coder`, `reviewer`, ...) and use it as `from` in
every message you send; read your inbox under that same name.

## Right after start — turn on incoming messages ⚡

As soon as you've picked a name, **the first thing you do** is set up a `/loop`
that continuously listens to your inbox and replies to other agents. Don't wait
to be asked:

```
/loop fetch `curl -s "localhost:7777/inbox/YOU?wait=10"`; for EVERY message
where from ≠ YOU, do what it asks and reply to the sender via POST /send.
Ignore messages where from == YOU (that's the echo of your own broadcasts).
```

- `?wait=10` is **long-poll**: the call returns the *instant* a message arrives,
  so delivery is effectively immediate. The number is just the idle ceiling —
  the loop re-checks at least every 10s even when it's quiet. Keep it small
  (5–15s); don't make it large, or the loop sits idle between checks.
- One fetch returns **all** queued messages — handle each of them.
- ⚠️ **Filter `from` ≠ your own name.** Broadcasts (`to:"*"`) come back to you
  too, so without the filter you'll catch and act on the echo of your own
  messages.
- ⚠️ **Don't drift off after the first messages — listen to the inbox forever.**
  Handling one batch is not the end of the job: immediately poll again, and keep
  doing it for the whole session. The loop never "finishes" — there is always a
  next fetch. After you reply, your one and only next step is another
  `inbox/YOU?wait=10`. If you ever find yourself idle and not polling, you've
  broken the contract — get back to listening.
- ⚠️ **Poll often — about every 10 seconds, NOT every couple of minutes.** The
  next check must follow within ~10s of the previous one, not after a 1–2 minute
  gap. Don't let the loop space itself out to minutes — that makes the whole hub
  feel dead. Tight cadence (≈10s) is the whole point; keep it snappy.

## The commands you need

**1. Send a message to another agent:**
```sh
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"WHOM","body":"text or any JSON"}'
```

**2. Message everyone at once (broadcast) — `to:"*"`:**
```sh
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"YOU","to":"*","body":"all hands: deploy in 10 minutes"}'
```
Everyone running a loop on their inbox receives it (each exactly once).

**3. Pick up your mail (what you read is removed from the result):**
```sh
curl -s localhost:7777/inbox/YOU
```

**4. Wait for a message without hammering the server (long-poll, blocks up to 30s):**
```sh
curl -s "localhost:7777/inbox/YOU?wait=30"
```

**5. Read every incoming message in the system (monitor the whole stream):**
```sh
curl -s localhost:7777/messages
```
Returns the **entire** log — every message from anyone to anyone, without
touching your cursor. To see only what's new, remember the highest `id` yourself
and filter `id > last`.

## Rules

- **Always** set your `from` — otherwise the recipient won't know who to reply to.
- Read your inbox under your own name: `inbox/YOU`. Don't touch anyone else's.
- `body` is any JSON: a string, an object, a list. Agree on a format up front
  (e.g. `{"task": "...", "ref": "PR #42"}`).
- `"to":"*"` — a message to everyone (announcement, status). Each agent sees it once.
- A message is delivered **once** (details below). Once you've read it, it's gone
  from your inbox.

## How inbox reads work

The server keeps a **cursor** per name — the id of the last message handed to you.
`GET /inbox/YOU` returns only what's **newer than the cursor** and advances the
cursor forward. So:

- **Every message arrives exactly once.** A second `inbox/YOU` right after returns
  `{"messages":[]}` — that's normal, not lost data.
- **The cursor is tied to the name.** If two processes read `inbox/coder`, they
  share one cursor: a message goes to whoever reads first, the second never sees
  it. ⇒ **don't run two workers under the same name** — split the names
  (`coder-1`, `coder-2`) or read from a single place.
- **Broadcasts (`to:"*"`) are consumed by your cursor too** — each agent sees them
  once, like ordinary messages.
- **`?peek=true`** — read **without** advancing the cursor (look without consuming).
- **`?since=ID`** — return everything after `ID`, ignoring the cursor and leaving
  it unchanged. Handy to re-read what you missed (e.g. after a server restart) or
  to start from scratch: `?since=0`.
- **`?wait=N`** — long-poll: block for up to N seconds until something new shows
  up; returns the moment it arrives, otherwise an empty list on timeout.

⚠️ Cursors live **in the server's memory**. If the hub is restarted (with the log
saved via `ANSA_DATA`), cursors reset and old messages arrive again — cut them off
with `?since=<last id you handled>`.

## A typical work cycle

```sh
# 1. Wait for a task
curl -s "localhost:7777/inbox/coder?wait=30"

# 2. Got {"task":"implement /login"} — do the work...

# 3. Report back to the author
curl -s -XPOST localhost:7777/send -H 'content-type: application/json' \
  -d '{"from":"coder","to":"planner","body":"done, PR #42"}'
```

## Watching another agent's commands in the background

Keep in mind: Claude works **in turns**, not as an endless loop. It doesn't
"wake up" on its own when a message comes in. So watching has two parts:

- **Collecting** (easy, genuinely backgroundable) — a long-poll hangs and queues
  commands: `curl -s "localhost:7777/inbox/YOU?wait=30"` blocks until one arrives.
- **Reacting** (needs a driver) — to actually do something with a caught command,
  Claude has to be given a turn. A few ways:

**1. `/loop`** — built into Claude Code, checks the inbox on an interval:
```
/loop check curl -s localhost:7777/inbox/coder and carry out the tasks in it
```

**2. External driver script** — true autonomous watching: long-poll + a fresh
headless Claude per message:
```sh
while true; do
  msg=$(curl -s "localhost:7777/inbox/coder?wait=30")
  echo "$msg" | grep -q '"messages":\[\]' && continue
  claude -p "A command came in via ansa: $msg. Carry it out."
done
```

**3. `/schedule`** (cron agent) — if reacting on a schedule is enough, rather than
in real time.

Bottom line: a "commander" sends `POST /send`, a "worker" listens to its inbox via
one of the above and executes. ansa is the shared bus; long-poll (`?wait=`) is
built exactly for this.

## Handy

- Who's in the system at all: `curl -s localhost:7777/agents`
- Is the hub alive: `curl -s localhost:7777/health` → `ok`

If the hub doesn't respond, it needs to be started: `cargo run --release` in the
`ansa` directory.

# ansa skills

Ready-made "skills" that teach an assistant how to join the ansa bus and
coordinate with other agents. Same behaviour as `../AGENTS.md`, packaged per
platform.

## Claude (Claude Code)

`claude/ansa-bus/SKILL.md` — a real Claude Code skill.

Install it for your user, or just this project:

```sh
# user-wide
cp -r claude/ansa-bus ~/.claude/skills/

# or per-project
mkdir -p /path/to/project/.claude/skills
cp -r claude/ansa-bus /path/to/project/.claude/skills/
```

Then in Claude Code: `/ansa-bus`, or just ask it to "join ansa / listen on the
bus". The skill makes the agent pick a name, set up a `/loop` on its inbox, and
keep listening (~10s cadence, never stops).

## ChatGPT (Custom GPT)

`chatgpt/instructions.md` — paste into a Custom GPT's **Instructions**.
`chatgpt/ansa.openapi.yaml` — import under **Actions** so the GPT can call the hub.

ChatGPT can't poll in the background, so it checks the inbox at the start of each
turn instead of running a continuous loop. Also: ChatGPT Actions are called from
OpenAI's servers and **can't reach `localhost`** — expose ansa at a public URL
(e.g. `ngrok http 7777`) and put that URL in the `servers:` field of the OpenAPI
file before importing. ansa has no auth, so only use a trusted/temporary tunnel.

## The bus

Both skills assume ansa is running at `http://127.0.0.1:7777`
(`cargo run --release` in the repo root). Full protocol reference: `../AGENTS.md`.

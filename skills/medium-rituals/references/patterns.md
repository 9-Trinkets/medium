# Medium rituals patterns

## Default stance

Use one ghost unless the user clearly wants a cast of characters. Keep behavior legible:

1. summon
2. speak
3. animate for the current phase
4. face toward work or toward delivery

## Good speech style

Prefer lines like:

- "On it."
- "Found the issue."
- "Making the change now."
- "Done."

Avoid lines that read like tool logs, plans, or file-path dumps.

## Animation choices

Map one animation to one phase when possible:

- `idle` for steady work
- `talk` for visible response
- ghost-specific emphasis animations for celebrations or alerts

Do not change animation on every tool call.

## Facing choices

Use simple defaults:

- face right while working
- face left while presenting or acknowledging the user

Keep the choice stable unless there is a meaningful shift in scene or tone.

## Multi-ghost scenes

Use multiple ghosts sparingly. When using them:

1. summon each ghost intentionally
2. target every action with `ghost=<name>`
3. keep one ghost primary so the scene stays readable

## Silence and dismissal

Not every step needs speech. Skip speech for trivial internal progress.

Dismiss a ghost only when the user asks or when the interaction clearly benefits from an exit beat.

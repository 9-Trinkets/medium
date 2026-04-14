---
name: medium-rituals
description: This skill should be used when the user asks to control Medium ghosts, "summon" a ghost, make a ghost "speak", play a ghost animation, set ghost facing, dismiss a ghost, or use a specific ghost like "vita" or a custom imported ghost.
---

# Run Medium rituals via MCP tools

Use Medium tools to make ghost interactions feel intentional, readable, and lightweight.

Keep the focus on embodiment, not raw command dispatch. Treat summon, speech, animation, and
facing as a small performance grammar for visible agent presence.

Keep the skill focused on runtime behavior:
- Speak early.
- Animate by phase, not by tool call.
- Use `ghost=` to target the right persona.
- Keep spoken lines short and natural.

## Setup expectations

Expect Medium to be installed locally and reachable through MCP.

Prefer the built-in integration commands when setup is missing:

```bash
medium init
medium integrate claude
medium integrate copilot
medium doctor
```

Use `medium integrate ... --ghost <name>` when a repo should default to a specific ghost.

## Core working rules

```gherkin
Feature: Voice Communication

  Rule: Speak first on non-trivial work

    Scenario: Starting a task that will use tools
      Given the user has asked for non-trivial work
      When I begin the task
      Then I call speak() before other Medium actions
      And the first line is short, warm, and conversational
      And the first line does not narrate the plan

  Rule: Speak again only at meaningful transitions

    Scenario: Moving from one phase of work to another
      Given I have finished a meaningful unit of work
      When I pivot to the next phase
      Then I may call speak() with a short update
      And I do not narrate every individual tool call

  Rule: Keep spoken text easy to deliver

    Scenario: Preparing any spoken line
      Given I am about to call speak()
      Then the text is plain sentences with no markdown
      And the text is under 30 words
      And the text avoids filenames, tool names, and jargon when possible

  Rule: Avoid duplicate written replies

    Scenario: Finishing a task after speaking during the work
      Given I have already spoken status updates
      Then the written reply should add results or conclusions
      And it should not repeat the same spoken wording
```

```gherkin
Feature: Animation Playback

  Rule: Start the animation at the start of a phase

    Scenario: Beginning investigation, implementation, or delivery
      Given I am starting a distinct phase of work
      When I choose an animation
      Then I play it before or as the phase begins
      And I match the animation to the work

  Rule: Use looping animations for sustained work

    Scenario: The phase spans multiple tool calls
      Given I need the animation to remain visible
      Then I use loop_anim=true

    Scenario: The action is brief or celebratory
      Given the animation is only a short accent
      Then I may use loop_anim=false

  Rule: Do not spam animations

    Scenario: Working through several tool calls in one phase
      Then I use at most one animation for that phase
```

```gherkin
Feature: Facing Direction

  Rule: Face toward the current intent

    Scenario: Working or investigating
      Then I usually face right

    Scenario: Presenting a result to the user
      Then I usually face left
```

```gherkin
Feature: Multi-Ghost Control

  Rule: Target ghosts explicitly when needed

    Scenario: More than one ghost is active
      Given multiple ghosts are present
      When I want to control a specific one
      Then I pass ghost=<name> to the Medium tool call

  Rule: Summon then speak immediately when needed

    Scenario: A newly summoned ghost should respond right away
      Given I have just called summon(name)
      Then it is valid to call speak(...) immediately after
      And I do not need to insert artificial delays
```

## Quick patterns

```python
summon("vita")
speak("On it.", ghost="vita")
play_animation("idle", loop_anim=True, ghost="vita")
set_facing("right", ghost="vita")
```

```python
speak("Found it. Making the change now.", ghost="vita")
play_animation("idle", loop_anim=True, ghost="vita")
```

```python
set_facing("left", ghost="vita")
speak("Done.", ghost="vita", voice=False)
```

## Available MCP tools

- `summon(name)`
- `dismiss(name)`
- `play_animation(name, loop_anim=False, ghost=None)`
- `set_facing(direction, ghost=None)`
- `speak(text, ghost=None, voice=None)`

## Additional resources

Consult these files when installation details or richer usage patterns are needed:

- `references/install.md` - Claude and Copilot setup flows
- `references/patterns.md` - practical interaction patterns and phrasing guidance

# PHASE 4 Plan: Popup GTK UI

## Current Baseline

- The engine already launches `emoji-input-ui`.
- The UI already receives `UpdateResults` over the session bus.
- Click-to-commit is already bridged back through the engine with a per-launch token.
- The popup is functional, but it is still a basic list-based window and needs polish.

## Phase 4 Goal

Turn the popup into a predictable, keyboard-friendly picker that feels native in GNOME without changing the engine contract again.

## Scope

- Keep the engine as the source of truth for composition state and search results.
- Keep the popup display-only; do not add a duplicate in-popup search entry.
- Improve popup layout, placement, and selection behavior.
- Make keyboard navigation explicit in the UI rather than relying on implicit listbox behavior.
- Add a clean hide/show lifecycle so the popup never gets stuck open.

## Suggested Order

1. Fix window positioning and visibility rules.
2. Add a dedicated search/display model inside the UI.
3. Wire explicit keyboard handling for up/down/enter/escape.
4. Tighten commit and dismissal behavior across the session-bus bridge.
5. Revisit styling only after the interaction model is stable.

## Non-Goals

- No new engine protocol.
- No settings migration yet.
- No variant picker redesign yet.

## Risks

- The popup is split across two processes, so lifecycle bugs can appear at the engine/UI boundary.
- The current token is instance-scoping, not authentication.
- Positioning behavior can drift depending on the compositor and active app.

# Phase 5 Plan - Recents + Variants

Phase 4 gives the popup a usable shape. Phase 5 should make the picker feel smarter without turning it into a different product.

Current implementation note: recents are now frequency-ranked in the engine, the popup exposes variants through a per-row chooser, and preferred variants are stored in GSettings. The remaining work is cache hardening and any UX edge cases that show up in real sessions.

## Recents

- Keep recents on the engine side so the popup stays stateless.
- Decide whether persistence stays in local files or moves to settings-backed storage.
- Sort recents by usage signal, not just last access.
- Cap the history so the list stays relevant.
- Make recents available to the UI as a first-class result section or a higher-priority search tier.

## Variants

- Use the existing variant list in the dataset as the source of truth.
- Add a light UI affordance for choosing variants when an emoji has them.
- Keep the default commit path simple when no variants are present.
- Avoid adding a separate variants database unless the dataset forces it.

## Cleanup

- Tighten the recents commit path so it does not regress the current search flow.
- Keep the engine as the owner of commit bookkeeping.
- Document any UI behavior changes once the interaction model is settled.

## Exit Criteria

- Recent emoji usage is visible in behavior.
- Variant selection works without breaking plain commits.
- The popup and engine still agree on the same commit contract.

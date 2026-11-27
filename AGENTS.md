# Rofi Hub agent instructions

## Required UI rules

These rules apply to every Rofi app in this repository and to apps added later. Read [the UI standard](docs/UI_STYLE.md) before changing a RASI theme, renderer, shortcut, or UI flow.

- Use the shared palette, typography, spacing, and selection styles. Choose each app's layout for its primary task. Calendar uses a month grid, Screen Time uses a timeline, and search-driven apps use a list.
- Show the primary content and routine actions on the same screen. Reveal details inline with one control per section. Reserve separate prompts for text entry that needs its own field, such as a password, and for confirmations such as clearing history.
- Give section headers a fixed appearance. Hover, focus, and click must never paint a selection highlight on a header. Headers must never run an action. Make headings, data rows, and actions distinguishable by more than color.
- Show the shortcut next to its action. A control styled as a button must work with a mouse. Check that shown shortcuts match the actual bindings and do not conflict with Rofi defaults or another action in the app.
- Use `Ctrl+1–4` for theme switching wherever the feature is offered. Switching theme changes only colors. Keep the configured font, popup geometry, and current input, selection, and view. Atomic Tasks uses `Alt+1–4` for priorities.
- Keep each app independently launchable. App state, theme overrides, and runtime dependencies belong to the app. The hub installer manages packages, and the optional menu launches them.

For UI changes, run the relevant existing checks and verify the changed behavior in Rofi with disposable data. Record what was checked and identify any live behavior still unverified. [Verification criteria](docs/UI_STYLE.md#screenshots-and-validation)

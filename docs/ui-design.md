# Viewer design

The viewer is a utility for comparing source sprites with material effects.
Its first task is to make a visual change easy to judge against a familiar
reference. Charizard, Cryogonal and Glalie are the default local examples;
previous generated examples remain on disk but are not loaded automatically.

## Layout and behavior

- Keep a short Pokémon list, two equally sized comparison panes, one shared
  playback row and one material inspector. No room titles, card gallery or
  introductory feature copy.
- Label the panes **Original** and **With materials**. Send the same animation
  time, view, zoom and placement to both renderers. Keep the original visible
  while effects change.
- The reference rendering bypasses material shading, material opacity, weather,
  bloom and tone mapping. Source animation opacity remains intact. This is a
  source-color reference, not merely the lit shader with its light turned down.
- Use ordinary text buttons, selects, checkboxes and ranges, a system font,
  neutral surfaces and one selection accent. Labels name the action or property.
- Keep routine controls visible. Put diagnostic maps, light budgets and
  reflection quality under **Advanced**. Preserve keyboard focus indicators,
  control labels, loading status and actionable error messages.
- Keep visual effects in the rendered scene. Cave illumination, emission, rain
  and reflections are the content under test, not decorations for the interface.

## Loading and measurements

The default Pokémon archives and thumbnails live in ignored `web/fixtures/`.
They are local source-art studies, not files included in the public collection.
If no fixtures are available, the viewer offers file import. Imported `.sprite`
packages join the comparison library; Save copy exports material adjustments.

Stress test draws the chosen population in each pane. Its count is per pane and
the browser submits both renders. The FPS label is display cadence, including
the comparison interface, rather than a completed-GPU-frame measurement.
Native benchmark records are separate. The 2026-11-09 mixed-world dataset uses
the retained original characters; it must not be relabeled as Pokémon results.
Earlier Pokémon records also retain their original inputs and renderer revision.

## Design guidance consulted

The actual skill documents below were read for guidance. No skill package or
design CLI was installed or run.

- [Impeccable: Operate](https://github.com/pbakaus/impeccable/blob/main/.agents/skills/impeccable/reference/operate.md)
  is the primary fit: familiar controls, readable density and predictable
  interactions for apps and editors.
- [Impeccable: Distill](https://github.com/pbakaus/impeccable/blob/main/.agents/skills/impeccable/reference/distill.md)
  supports consolidating repeated controls, removing unnecessary containers
  and revealing advanced settings when needed.
- [UI-Deslopify](https://github.com/AntonioSpagnol/UI-Deslopify-Skill/blob/main/SKILL.md)
  and its [audit rules](https://github.com/AntonioSpagnol/UI-Deslopify-Skill/blob/main/AUDIT-RULES.md)
  identify ornamental gradients, glass panels, excessive pills and generic
  feature cards. Its branding intake and component-marketplace workflow were
  not adopted for this local tool.
- [deslop-ui](https://github.com/viktorbezdek/skillstack/blob/main/deslop/skills/deslop-ui/SKILL.md)
  informs short action labels and direct messages without promotional filler.

# Depth and weather experiments

These are renderer/scene experiments, not new required fields in the sprite
format. Games own placement, cameras, weather, and which characters are nearby.
The source package still owns editable drawings, named views, animation, and
material maps.

## Face-to-face battle

The **Pair** view renders the opponent's real Front rig in the upper right and
our Pokémon's real Back rig in the lower left. Both receive full material shading
and evaluate their animations on the same playback clock. The selectors above
the stage choose either participant; **Swap sides** exchanges their roles. The
Night, Rain, Studio, and Reflections environments work with this layout. Scene
changes keep the chosen pair, and the URL retains the view, environment and pair.
Specialist controls are in expandable Materials, Glow, Test objects and Playback
sections; the common scene and emission/reflection switches stay above the stage.

The oblique orthographic camera and two finite-depth card positions share a single
projection with the actual rig placement. Each receiver excludes itself from
reflection rays. In the far Pokémon's camera-facing surface, the near Pokémon's
Front capture can be reflected even though its visible rig uses Back. Swapping
sides supplies the converse pairing. The near Pokémon's visible back does not
reflect the distant opponent through itself; it can see a camera-side trainer
marker. Trainer markers are reflection references, hidden from primary battle
visibility. View → Reflected scene shows the incoming color before material tint.

Rain mirrors both animated views into the wet floor. Cave emission is gathered
from both rigs and illuminates the procedural surroundings. These remain bounded
preview approximations, not a gameplay combat system or recursive light transport.

Open `?view=pair&asset=emberwing&player=emberwing&scene=rain` (or `scene=cave`).

## Renderer depth controls (legacy inspector)

The simplified UI presents Sprite, Pair and World. The close-up/depth controls
below describe the retained renderer settings and historical inspector; they
are no longer separate menus in the main viewer.

## What a reflection sees

A front-facing opponent can reflect the **front** of the player's Pokémon even
though the battle camera sees that Pokémon's **back**. The visible back surface
of the player's Pokémon can reflect its trainer, who is on the camera side.
Objects beyond a flat surface do not magically appear in its reflection;
transmission and reflections from sharply sloping facets are separate cases.

The lab now casts a reflection ray in XYZ. It intersects finite-depth cards,
rejects hits behind the ray, chooses the closest nontransparent hit, and selects
Front or Back according to the side from which the card is seen. The opposite
view is mirrored in the corresponding coordinate direction. Reflected artwork
continues animating even when hidden from the direct camera.

- **Close-up:** hides foreground figures from primary visibility to inspect the
  selected surface. They remain present to reflection rays. The camera is normal
  to the sprite plane.
- **Battle camera:** uses an oblique orthographic projection. Near and far objects
  appear at different screen positions; the near Pokémon shows its back.
- **Depth (Z):** moves the source onto the camera side or the far side. Front/Back
  changes the inspected side. The diagram reports the resulting placement.
- **Inspect the other Pokémon:** exchanges the selected surface and source and
  flips the viewing side. This lets the player's visible back be inspected.
- **Trainer markers:** original procedural front/back markers behind the two
  positions; these are test references, not game trainer artwork.
- **Reflected scene:** shows incoming reflected color before material tint and
  Fresnel strength, making the selected direction easy to inspect.

Coordinates use +Z toward the active camera; one XY world unit is one viewport
height. The demonstration treats views named Front and Back as opposing sides,
falling back to the first available view. This is a lab convention, not a frozen
profile for all games.

Four 256×256 captures (selected/source × front/back) share one RGBA16F texture.
Captures use the real articulated poses and drawing swaps; transparent texels
stay transparent. They contain original color plus emission, without recursively
rendering reflections. In the single-sprite Depth inspector, only the selected sprite receives material
shading and the other figures are reference cards. In Battle, both visible rigs
receive material shading; reflected captures still contain one bounce of color
and emission rather than recursive lighting. There is no reconstructed volume, side-view interpolation, geometric
self-shadowing, or physically calibrated thickness.

## Rain

The scene includes a cloudy sky, distant terrain, rain behind and in front of the
sprite, wind, animated ground ripples, and a wet floor. Rain and animation share
the playback clock; pausing or scrubbing also controls the weather.

Wetness reduces effective roughness, perturbs highlights, and controls the wet
floor reflection. These are scene effects rather than edits to the asset's maps.
Single-sprite and Battle floors use the animated color/emission captures with a
vertical mirror and ripple distortion. The reflection switch disables these floor
mirrors too. This planar approximation is disabled for crowds.
Rain does not extinguish fire, simulate water volumes, or alter gameplay fields.

## Measurement

Capturing all four views is bounded work, included in native frame timing. Rain,
depth, and Battle use seven draw calls (the six normal passes plus one capture pass) and
an additional fixed 2 MiB texture. Empty captures are cleared, so hidden drawings
cannot leave an old reflection behind. Crowds share the same source cards; the
benchmark does not imply thousands of independent opponents or light probes.

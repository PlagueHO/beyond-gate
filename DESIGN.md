---
name: Beyond Gate documentation
description: Gatehouse visual system for the production documentation website.
colors:
  action: "#b51c27"
  action-hover: "#97131e"
  action-text: "#ffffff"
  dark-accent: "#ff7778"
  dark-accent-hover: "#ffa0a1"
  dark-bg: "#17191c"
  dark-alt: "#111316"
  dark-soft: "#202328"
  dark-text: "#f5f3f0"
  dark-muted: "#bdbbb8"
  dark-divider: "#3e4146"
  light-bg: "#f9fafb"
  light-alt: "#eceef1"
  light-soft: "#eff1f4"
  light-text: "#262222"
  light-muted: "#595150"
  light-divider: "#d3cdca"
typography:
  body:
    fontFamily: '"Segoe UI", "Helvetica Neue", Arial, sans-serif'
    fontSize: "17px"
    fontWeight: 400
    lineHeight: 1.8
  body-mobile:
    fontFamily: '"Segoe UI", "Helvetica Neue", Arial, sans-serif'
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.8
  action-label:
    fontFamily: '"Segoe UI", "Helvetica Neue", Arial, sans-serif'
    fontSize: "15px"
    fontWeight: 650
  command:
    fontFamily: 'Consolas, "Liberation Mono", monospace'
    fontSize: "13px"
rounded:
  control: "2px"
spacing:
  compact: "8px"
  body: "16px"
  group: "24px"
  section: "32px"
  wide: "48px"
  entrance: "64px"
components:
  action-primary:
    backgroundColor: "{colors.action}"
    textColor: "{colors.action-text}"
    typography: "{typography.action-label}"
    rounded: "{rounded.control}"
    padding: "13px 20px"
  action-primary-hover:
    backgroundColor: "{colors.action-hover}"
  action-secondary:
    typography: "{typography.action-label}"
  text-link:
    typography: "{typography.action-label}"
  command-example:
    rounded: "{rounded.control}"
    padding: "16px 14px"
  navigation-link:
    padding: "0 12px"
    height: "64px"
---

# Design System: Beyond Gate documentation

## Overview

**Creative North Star: "Gatehouse"** guides this website.

This system covers the production documentation website only. It does not govern
the native Rust CLI, terminal output, or MCP responses. Original red gate artwork
and a white and red wordmark carry the fantasy identity. Quiet reading surfaces,
clear links, and restrained borders support long references.

Gatehouse is the autonomous production default, not an explicitly approved visual
direction. Dark mode opens by default; readers can select light mode. The confirmed
brief asks for a fantasy feeling related to D&D Beyond without copied artwork or
branding. Five working comparison studies remain separate from this system.

**Key Characteristics:**

- Original red, white, and black identity assets.
- Dark and light reading surfaces with theme-aware link accents.
- Flat sections, solid dividers, and compact control corners.
- Responsive reference navigation with local search.

Evidence comes from `PRODUCT.md`, `docs/index.md`, and the production files
`docs/.vitepress/theme/theme.css`, `SiteLayout.vue`, `DocsOverview.vue`, `index.ts`,
and `docs/.vitepress/config.ts`. Installed VitePress (1.6.4) supplies the base
layout and controls. Final supplied captures cover overview and CLI at widths
(1440px) and (390px), plus the light overview at (1440px).

No approved rendered comp or quality-bar card exists. No random concept seed was
used. The supplied finish-review handoff reports no material UI blockers after
the mobile and wordmark fixes. This record does not claim visual approval or
independent test execution.

## Colors

Crimson actions and warm light lettering sit on cool charcoal in dark mode.
Light mode uses pale reading grounds with dark text and deeper crimson links.
The frontmatter records a small reused subset, not the full VitePress palette.

### Primary

- **Action Crimson** (`action`): Filled installation action in both modes.
- **Deep Crimson** (`action-hover`): Filled action hover and light link hover.
- **Gate Ember** (`dark-accent`): Dark-mode links, selected navigation, focus,
  and the emphasized overview line.
- **Pale Ember** (`dark-accent-hover`): Dark-mode text-link hover.

Light mode maps its link accent to Action Crimson and its hover to Deep Crimson.
The filled action stays crimson rather than adopting the lighter dark-mode accent.

### Neutral

- **Action White** (`action-text`): Text on the filled action.
- **Gate Charcoal** (`dark-bg`): Dark reading ground and permanent wordmark backing.
- **Deep Charcoal** (`dark-alt`): Dark sidebar and command-example ground.
- **Soft Charcoal** (`dark-soft`): Dark alternating table rows.
- **Warm White** (`dark-text`): Dark-mode primary text.
- **Stone Text** (`dark-muted`): Dark-mode descriptions and secondary text.
- **Charcoal Divider** (`dark-divider`): Dark-mode section and container borders.
- **Reading White** (`light-bg`): Light reading ground.
- **Pale Slate** (`light-alt`): Light sidebar, table headers, and command examples.
- **Soft Slate** (`light-soft`): Light alternating table rows.
- **Ink Text** (`light-text`): Light-mode primary text.
- **Muted Ink** (`light-muted`): Light-mode secondary text.
- **Stone Divider** (`light-divider`): Light-mode section and container borders.

**The Mark Backing Rule.** Keep the white and red wordmark on its permanent
charcoal backing in both modes.

Sidecar tonal ramps are synthetic swatch metadata derived from these colors.
They are not installed CSS tokens, approved color pairs, or permission to expand
the production palette.

## Typography

**Body Font:** Segoe UI, with Helvetica Neue, Arial, and sans-serif fallbacks.
**Label/Mono Font:** Consolas, with Liberation Mono and monospace fallbacks.

Body copy uses familiar text shapes and generous leading. Commands remain mono.
The source also assigns the body stack to display headings. That system-display
choice is recorded as carried craft-floor drift, not a new reusable display rule.
No separate display-face token is established here.

### Hierarchy

The following heading measurements describe the shipped build, not a mandate to
extend its system-display choice:

- **Overview heading:** Fluid desktop size (`clamp(2.6rem, 4.3vw, 4.2rem)`),
  weight (650), leading (1.08), tracking (-.035em). Mobile uses
  `clamp(2.3rem, 8vw, 3.3rem)`.
- **Reference h1:** Size (2.7rem), weight (600), leading (1.12), tracking (-.03em).
  Mobile uses (2.2rem).
- **Reference h2:** Size (1.8rem), weight (600), leading (1.2).
- **Reference h3:** Size (1.35rem), weight (600), inherited leading (28px).
- **Overview section heading:** Size (30px), leading (1.2), tracking (-.025em).
  Mobile uses (27px). Interface-path titles use (20px), weight (650).
- **Reference body:** Frontmatter `body`, then `body-mobile` below the mobile
  breakpoint. Paragraphs and list items have a maximum line length (74ch).
- **Overview description:** Size (19px), leading (1.7), maximum width (44ch).
  Mobile uses (17px). Supporting section copy uses (16px).
- **Action labels:** Frontmatter `action-label`. Top navigation uses (14px),
  weight (500), then (700) when active.
- **Command sample:** Frontmatter `command`. Its descriptive label uses (12px)
  secondary text. Reference tables use (14px), leading (1.65).

## Layout

The overview uses a centered container (1240px maximum) with desktop padding
(64px 48px 80px). Its entrance pairs copy and artwork in columns (1.35fr / 1fr),
with a gap (64px), minimum height (480px), and lower separation (64px).
This describes the installed overview, not a template for every reference page.

The interface section uses columns (1fr / 2fr) with a gap (56px).
The two interface paths form equal columns with a gap (32px). Section spacing
uses the recorded rhythm with local values where content requires them.
At widths up to (1100px), side padding and major gaps reduce to (32px).

At widths up to (767px), overview padding becomes (32px 24px 48px). The entrance
stacks copy above the gate figure, with a gap (28px) and no minimum height.
The figure becomes a horizontal strip with padding (20px), gap (24px), and a
non-shrinking gate basis (100px). Interface paths become one column. Supporting
sections stack with vertical padding (34px).

Reference content has a maximum container width (790px). VitePress retains its
header height (64px), persistent sidebar from (960px), and desktop top navigation
from (768px). Narrow screens expose menu and page-outline controls. The desktop
reference capture includes the right page outline; the config includes heading
levels (2) and (3). The base page minimum width is (320px).

## Elevation & Depth

Custom production overview components use no box shadows. Darker command grounds,
solid borders, and section dividers establish depth. The desktop gate outline
frames the existing artwork without a filled card. VitePress owns local-search
and navigation overlay behavior; this is not a global ban on its overlay shadows.

**The Flat Reading Rule.** Use tonal grounds and solid dividers for the custom
reading sections rather than adding decorative elevation.

## Shapes

Controls and command containers use the compact frontmatter corner radius.
Custom borders are solid (1px). The desktop gate figure is the signature exception,
with an arched radius (180px 180px 3px 3px); mobile reduces it to (2px).
This arch frames the gate asset, not every content container.

Keyboard focus uses the current theme accent in a solid outline (3px), offset
(4px), with corners (2px). The shared rule covers links, buttons, inputs, selects,
and focusable elements. Do not substitute dashed or dotted outlines.

## Components

### Actions and text links

The primary installation link is a compact crimson slab. Its border matches the
resting fill, its label uses Action White, and its padding comes from frontmatter.
Hover changes the background only, with a transition (0.18s ease-out).

The secondary action has no filled container. It inherits the current text color,
uses an underline offset (4px), and adopts the current theme accent on hover.
Reference links in the custom overview use the theme accent at rest and the
second brand shade on hover. All use the shared solid keyboard focus treatment.
The custom source defines no distinct pressed or disabled action state.

### Command containers and reading tables

Command examples are static reading aids, not inputs or status cards. They use
the alternate ground, a divider border, compact corners, a descriptive text label,
and horizontally scrollable single-line code. They have no hover or focus state.

Reference code blocks keep VitePress syntax rendering with a custom divider border
and compact corners. Tables use the alternate ground for headers and the soft
ground for alternating rows. Header weight is (650).

### Navigation and search

VitePress supplies top navigation, the sidebar, mobile menus, page outlines,
local-search dialog, theme switch, and previous/next links. The custom title adds
the original gate mark (34px). Active top links and active sidebar entries gain
weight (700); the active top link uses the theme accent.

Navigation keeps overview, installation, CLI, MCP, and development routes.
Sidebar groups separate user references from development. Local search remains
the installed VitePress control, not a new form design. `SiteLayout.vue` returns
focus to the search trigger after Escape closes the local-search box.

### Identity figure

Use the existing assets rather than redrawing the mark or replacing the wordmark
with a font. The overview wordmark has a desktop width (190px), internal padding
(10px), and permanent charcoal backing. Mobile width is (140px). The desktop
gate image has a maximum width (280px); mobile keeps (100px).

### Motion and specimen limits

Reduced-motion mode removes custom transitions and restores automatic scrolling.
The sidecar contains five source-derived, self-contained specimens: primary
action, secondary action, reference link, command container, and top navigation.
They bind to production custom properties with dark-default fallbacks.
Text-only action specimens omit the shipped decorative arrow glyphs. They do
not invent replacement artwork, custom forms, validation states, or theme-study
controls.

Not canonized or repaired: system display faces and decorative arrow glyphs are
carried craft-floor drift. They are evidence of the build, not styles for future
surfaces to inherit. This documentation-only handoff leaves UI unchanged.

## Do's and Don'ts

### Do

- **Do** use the original Beyond Gate identity assets and retain the wordmark backing.
- **Do** select the dark or light semantic colors as a set.
- **Do** preserve solid keyboard focus and the reduced-motion behavior.
- **Do** retain readable line lengths, scrollable commands, and mobile stacking.
- **Do** keep user references distinct from development navigation.

### Don't

- **Don't** treat comparison studies as production tokens or components.
- **Don't** present the autonomous Gatehouse default as explicit visual approval.
- **Don't** infer form, validation, or status variants from static command examples.
- **Don't** use dashed or dotted UI outlines.
- **Don't** turn the observed display-face and glyph drift into reusable rules.

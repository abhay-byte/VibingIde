# Design System Specification: The Kinetic Syntax

## 1. Overview & Creative North Star
**Creative North Star: "The Ghost in the Machine"**

This design system rejects the clunky, static nature of traditional development environments. Instead of a rigid grid of boxes, we are building a high-performance, "Hacker-Chic" terminal-inspired interface that feels fluid, intentional, and editorial. 

The aesthetic is driven by **Kinetic Layering**—the idea that the IDE is not a flat plane, but a series of translucent, illuminated data-strata. We break the "standard" look by using extreme typographic contrast (pairing technical monospace with elegant sans-serif) and replacing heavy structural lines with tonal shifts and light-leak accents. The result is a workspace that feels like a premium command center—precise, dark, and hyper-focused.

---

## 2. Color & Surface Architecture
We utilize a "Deep Ink" palette designed to minimize eye strain while maximizing the "pop" of technical syntax.

### The Palette
- **Core Background:** `surface` (#131313) - The void from which code emerges.
- **Primary Accent:** `primary_fixed` (#56ffa7) - A neon-electric green for active execution states.
- **Secondary Accent:** `secondary_fixed` (#b7eaff) - A data-blue for structural UI elements.
- **Tertiary Accent:** `tertiary_fixed` (#ffdcbb) - A sunset orange for warnings and "human-in-the-loop" agent panels.

### The "No-Line" Rule
Standard 1px solid borders are strictly prohibited for layout sectioning. In this system, boundaries are "felt, not seen." Use background shifts:
- A PTY terminal panel should use `surface_container_lowest` (#0e0e0e) to "recede" into the background.
- A floating command palette should use `surface_bright` (#393939) to "lift" toward the user.

### Glass & Gradient Signature
To achieve the "Hacker-Chic" depth, sidebars and utility panels must use **Subtle Glassmorphism**:
- **Background:** `surface_variant` (#353534) at 60% opacity.
- **Effect:** `backdrop-blur: 20px`.
- **Accent:** A 1px "Ghost Border" using `outline_variant` (#3b4b3f) at 15% opacity on the leading edge only to catch the light.

---

## 3. Typography
The system relies on a high-contrast pairing between human-readable UI and machine-perfect code.

| Level | Token | Font Family | Size | Intent |
| :--- | :--- | :--- | :--- | :--- |
| **Display** | `display-sm` | Space Grotesk | 2.25rem | Project titles / Empty states |
| **Headline** | `headline-sm` | Space Grotesk | 1.5rem | Panel headers |
| **Title** | `title-sm` | Inter | 1.0rem | File names / Tab labels |
| **Body** | `body-md` | Inter | 0.875rem | Documentation / UI labels |
| **Code** | `custom-mono` | JetBrains Mono | 0.875rem | Editor / CLI / PTY output |
| **Label** | `label-sm` | Inter | 0.6875rem | Micro-metadata / Line numbers |

**Editorial Note:** Use `Space Grotesk` for high-level branding and `Inter` for functional UI. Always render JetBrains Mono with `font-feature-settings: "liga" 1` to ensure code ligatures are razor-sharp.

---

## 4. Elevation & Depth
We eschew traditional drop shadows for **Tonal Layering**. Depth is a gradient of luminosity, not a projection of black ink.

- **Level 0 (Base):** `surface` (#131313). Used for the main editor gutter.
- **Level 1 (Inset):** `surface_container_lowest` (#0e0e0e). Used for integrated terminals to create a "pit" look.
- **Level 2 (Raised):** `surface_container_low` (#1c1b1b). Used for inactive sidebars.
- **Level 3 (Floating):** `surface_bright` (#393939) + **Ambient Shadow**.
- **The Ambient Shadow:** Shadows must be `rgba(0, 227, 138, 0.04)` (a tinted primary glow) with a 40px blur. This makes floating elements appear as if they are backlit by the code itself.

---

## 5. Components

### The "Pulse" Button (Primary)
- **Base:** `primary_container` (#00ff9c).
- **Text:** `on_primary_container` (#007142).
- **Style:** Square corners (`none`) or `sm` (0.125rem). Roundness is for consumers; sharp edges are for creators.
- **State:** On hover, apply a `surface_tint` outer glow.

### Technical Input Fields
- **Background:** `surface_container_highest`.
- **Border:** None. Use a 2px bottom-border of `outline_variant` that transitions to `primary` on focus.
- **Font:** Always `body-md` (Inter) for labels, but the typed text should be `JetBrains Mono` to emphasize the "input" as data.

### Agent Status Chips
- **Container:** `tertiary_container` (Sunset Orange).
- **Typography:** `label-sm` All-Caps.
- **Detail:** Add a 4px solid circle of `on_tertiary_fixed` to the left of the text to represent a "heartbeat" status.

### The PTY Panel (Terminal)
- **Separation:** Forbid dividers. Use a `spacing-2` (0.4rem) gutter between panels.
- **Header:** Use `surface_container_high` with `label-md` typography.
- **Active State:** The active panel is denoted by a `primary` (#f3fff3) top-border of 2px; inactive panels have 0px borders.

---

## 6. Do’s and Don'ts

### Do:
- **Use Asymmetry:** Place metadata (line counts, git branches) in unexpected but balanced locations to break the "header-body-footer" monotony.
- **Embrace Breathing Room:** Use `spacing-8` (1.75rem) around major PTY groups. High-performance tools need "oxygen" to prevent cognitive overload.
- **Leverage Tonal Shifts:** Change background colors to indicate focus rather than drawing a box around a focused element.

### Don’t:
- **Don't use 100% white (#FFFFFF):** It causes halation on dark backgrounds. Use `primary` (#f3fff3) or `on_surface` (#e5e2e1) for text.
- **Don't use Rounded-XL corners:** Keep the "Hacker-Chic" vibe by sticking to `none` or `sm` (0.125rem) radius. Softness is the enemy of precision.
- **Don't use Dividers:** If you feel the urge to add a line, add `0.4rem` of empty space or shift the background hex code by one step instead.

---

## 7. Interaction Intent
When a user hovers over a file in the tree, do not show a gray box. Instead, shift the background to `surface_container_low` and change the text color to `primary_fixed`. This "lighting up" effect mimics a terminal cursor moving through lines of code, reinforcing the high-performance, kinetic nature of the system.
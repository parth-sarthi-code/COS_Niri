# COS_Niri Quick Settings - Visual & Layout Defect Analysis & Fix Plan

## Executive Summary

A detailed visual audit of the rendered Quick Settings panel reveals several significant CSS styling, color contrast, grid geometry, and GTK theme inheritance bugs when compared to the authentic ChromeOS Quick Settings design.

The primary issue is that **GTK4's default widget stylesheet (Adwaita/HighContrast) is overriding custom button classes**, causing circular buttons (Header icons, Tile bubbles, Slider buttons) to render with solid white backgrounds (`#ffffff`) and white icons (`#ffffff`), rendering icons completely invisible. Additionally, the grid layout is rendered as **2 columns x 3 rows** instead of ChromeOS's **3 columns x 2 rows**.

---

## Detailed Visual Defect Breakdown

### 1. Header Area Defect
- **White-on-White Action Buttons**: The Header action buttons (Power, Lock, Settings, Collapse) render as solid white circles (`#ffffff`) with white icons inside (`#ffffff`), making the icons totally invisible.
- **Sign Out Button Low Contrast**: The "Sign out" pill button renders with faint grey text on a white/translucent background, failing accessibility standards.
- **ChromeOS Target**: Action buttons should be subtle translucent dark surfaces (`rgba(255, 255, 255, 0.08)`) with crisp light icons (`#e3e2e6`) and smooth hover feedback (`rgba(255, 255, 255, 0.16)`).

### 2. Grid Geometry & Feature Tile Defect
- **Incorrect Grid Layout (2x3 vs 3x2)**: The current grid is attached as 2 columns x 3 rows. ChromeOS Quick Settings uses **3 columns x 2 rows** (Row 0: Wi-Fi, DND, Capture; Row 1: Bluetooth, Night Light, Cast).
- **Solid White Tile Bubbles**: Every feature tile bubble renders as a solid white circle (`#ffffff`), hiding the white Material Symbols icons (`#ffffff`).
- **Inconsistent Tile Active States**: Active tiles (e.g. Wi-Fi connected, Bluetooth ON) should use a soft Material You blue (`#b4c5ff`) with dark icons (`#1a1b38`), whereas inactive tiles should use a dark surface (`rgba(255, 255, 255, 0.08)`) with light icons (`#e3e2e6`).
- **Label & Dropdown Arrow Spacing**: The title labels ("RUBYX", "Bluetooth") and sub-page dropdown arrows (`arrow_drop_down`) are cramped and misaligned.

### 3. Sliders Section Defect
- **Solid White Slider Icon Buttons**: The Volume Mute icon, Volume Sub-page arrow, and Brightness Sun icon render as solid white circles (`#ffffff`) with white icons (`#ffffff`).
- **Slider Track & Knob Padding**: GTK Scale sliders (`scale.qs-slider`) inherit default GTK slider troughs with hard edges instead of ChromeOS's pill-shaped track highlight and smooth thumb knob.

### 4. Container & Glassmorphism Defect
- **Width & Margin**: Panel width (380px) is compressed, causing grid items to wrap awkwardly.
- **Blur & Transparency**: Background transparency is missing proper CSS resets for nested GTK containers, allowing standard GTK dark borders to leak through.

---

## Root Cause Technical Diagnosis

1. **GTK4 CSS Selector Specificity**:
   Standard GTK button rules (`button`, `button:not(...)`) in GTK4 have higher specificity than simple `.qs-tile-bubble` or `.qs-header-icon-btn` classes. Without explicit `background-image: none; background-color: ... !important;` resets, GTK applies default button drawing code.

2. **Grid Coordinate Mapping in `grid.rs`**:
   `grid.attach(&tile, col, row, 1, 1)` mapped items using `(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)`, resulting in a 2-column, 3-row layout instead of `(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (2, 1)`.

3. **Material Design 3 Theme Tokens**:
   Material Symbols labels require strict `color` specification per active/inactive state so that dark icons render on light blue bubbles and light icons render on dark surface bubbles.

---

## Comprehensive Blueprint for Fix (Fix Plan)

### Step 1: CSS Resets & Specificity Overhaul (`src/style.css`)
- Add high-specificity CSS resets for all Quick Settings buttons (`button.qs-header-icon-btn`, `button.qs-tile-bubble`, `button.qs-slider-icon-btn`, `button.qs-slider-arrow-btn`).
- Force `background-image: none;`, explicit `background-color`, `border: none;`, `box-shadow: none;`.
- Define explicit active/inactive tile states:
  - **Inactive**: `background-color: rgba(255, 255, 255, 0.08); color: #e3e2e6;`
  - **Active**: `background-color: #b4c5ff; color: #1a1b38;` (dark icon on pastel blue).
- Style GTK Scale troughs (`scale.qs-slider trough`, `scale.qs-slider highlight`, `scale.qs-slider slider`) with ChromeOS rounded track geometry.

### Step 2: Grid Layout Correction (`src/components/quick_settings/grid.rs`)
- Re-align grid column/row parameters for exact **3 columns x 2 rows**:
  - Row 0: Wi-Fi `(0, 0)`, DND `(1, 0)`, Screen Capture `(2, 0)`
  - Row 1: Bluetooth `(0, 1)`, Night Light `(1, 1)`, Cast `(2, 1)`
- Fix title label centering and dropdown arrow placement inside feature tiles.

### Step 3: Header & Slider Icon Contrast (`header.rs`, `sliders.rs`)
- Update label color bindings so icon labels inside circular buttons inherit parent button text color (`color: inherit;`).
- Adjust panel width to 410px for spacious, authentic ChromeOS proportions.

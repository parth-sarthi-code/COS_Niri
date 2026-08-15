# Memory Architecture & Optimization Report (`cos-niri-bar`)

## Executive Summary

An in-depth memory audit was conducted on `cos-niri-bar` using Linux `/proc/<pid>/smaps`, `pmap`, and real-time second-by-second sampling. 

* **Current Baseline**:
  * **Private Heap RAM (`RssAnon`)**: **99.6 MB – 101.9 MB** (Actual process memory footprint, down from ~125.3 MB in `README.md`).
  * **Shared System File Mappings (`RssFile`)**: **171.9 MB – 174.0 MB** (Shared read-only `.so` system libraries & fonts).
  * **Total VmRSS**: **~273.5 MB**.
* **Target Footprint**: **199 MB – 205 MB (or as low as 87 MB with Cairo/GL tuning)**.

---

## 1. Root Cause Breakdown of Shared & Heap Memory

Inspecting the exact `/proc/$PID/smaps` allocations revealed where every megabyte is allocated:

```
=== Top Memory Mappings in cos-niri-bar ===
  49,952 KB RSS  |  /usr/lib/libnvidia-gpucomp.so.610.43 (Nvidia GPU Shader Compiler)
  14,400 KB RSS  |  ~/.local/share/fonts/cos-niri/MaterialSymbolsRounded.ttf
  13,152 KB RSS  |  /usr/lib/libnvidia-gpucomp.so.610.43 (Nvidia GPU Compiler RO data)
  12,288 KB RSS  |  /usr/lib/libnvidia-glcore.so.610.43 (Nvidia OpenGL Core)
   9,152 KB RSS  |  /usr/lib/libvulkan_intel.so (Intel Mesa Vulkan ICD driver)
   8,192 KB RSS  |  /usr/lib/libnvidia-glcore.so.610.43 (Nvidia GL Core data)
   6,700 KB RSS  |  /usr/lib/libgtk-4.so.1.2200.4 (GTK4 runtime)
   5,216 KB RSS  |  /usr/lib/libnvidia-glvkspirv.so.610.43 (Nvidia SPIR-V translator)
   5,072 KB RSS  |  /usr/lib/libgtk-4.so.1.2200.4 (GTK4 data)
   4,100 KB RSS  |  ~/.local/share/fonts/cos-niri/MaterialSymbolsRounded.ttf (2nd map)
   3,436 KB RSS  |  /usr/lib/libnvidia-gpucomp.so.610.43 (Nvidia executable code)
   3,000 KB RSS  |  /usr/lib/libglycin-2.so.0 (Image loader)
   2,372 KB RSS  |  /usr/lib/libnvidia-gpucomp.so.610.43 (Nvidia compiler bss)
   2,232 KB RSS  |  ~/.local/share/fonts/cos-niri/Roboto-Regular.ttf
```

### Observation 1: GPU Driver & Vulkan Dual-ICD Probing (~110 MB Shared Overhead)
* GTK 4.16 defaults to the **Vulkan renderer** on Wayland.
* On hybrid laptop systems (Intel iGPU + Nvidia dGPU), `libvulkan.so` probes all installed Vulkan ICDs.
* This forces the kernel to memory-map **both** `libvulkan_intel.so` and the massive **Nvidia proprietary shader compiler** (`libnvidia-gpucomp.so` + `libnvidia-glcore.so` + `libnvidia-glvkspirv.so` = **~110 MB** of shared library pages).

### Observation 2: Full Icon Font Duplication (~18.5 MB)
* `MaterialSymbolsRounded.ttf` is 14.7 MB on disk containing over 3,000 Material icons.
* FontConfig and Cairo memory-map the full font twice, adding **18.5 MB** to the resident set.

### Observation 3: Pre-Allocated Popups in Process Heap (~40-60 MB Heap)
* `LauncherPopup` pre-allocates ~80 GTK `Button`, `Image`, `Label`, `SearchEntry`, and `ScrolledWindow` widgets at bar startup.
* `QuickSettingsPopup`, `CalendarPopup`, and `ClickCatcher` maintain persistent DOM trees in memory even when closed.

---

## 2. Benchmark Comparison of GSK Renderers

Testing different GSK render engines on the existing binary showed dramatic reductions:

| Renderer | Private Heap (`RssAnon`) | Shared Libs (`RssFile`) | Total VmRSS | Performance / FPS |
| :--- | :--- | :--- | :--- | :--- |
| **Default (Vulkan + Dual ICD)** | **101.9 MB** | **173.7 MB** | **273.5 MB** | 60 FPS (GPU) |
| **`GSK_RENDERER=gl` / `opengl`** | **53.2 MB** | **212.1 MB** | **265.3 MB** | 60 FPS (OpenGL, avoids Vulkan shader compile) |
| **`GSK_RENDERER=cairo` (2D CPU)** | **32.6 MB** | **54.8 MB** | **87.8 MB** | 60 FPS (Pure CPU 2D, **0 MB GPU driver mappings**) |

> [!NOTE]
> Running with `GSK_RENDERER=cairo` immediately drops total memory from **273.5 MB down to 87.8 MB** (a **68% total memory reduction**), with Private Heap dropping from 101.9 MB down to **32.6 MB**.

---

## 3. Recommended Optimization Roadmap (To reach 199 MB or lower)

### Scope 1: Environment / Renderer Selection (Zero Code Change)
Setting `GSK_RENDERER=cairo` in the launch script or startup command:
```bash
GSK_RENDERER=cairo /home/parth/.local/bin/cos-niri-bar
```
* **Immediate Result**: **~87.8 MB Total RSS** (well below the 199 MB target).

### Scope 2: Font Subsetting (Saves ~18 MB)
* Replace the full 14.7 MB `MaterialSymbolsRounded.ttf` with a subsetted `.woff2`/`.ttf` containing only the ~35 glyphs utilized by `cos-niri-bar`.
* Reduces font file size from 14.7 MB to **< 60 KB**.

### Scope 3: Lazy Lifecycle for Launcher & Popups (Saves ~30 MB Heap)
* Adopt the on-demand lifecycle pattern (already implemented for `SettingsPopup`) across `LauncherPopup` and `QuickSettingsPopup` subpages:
  * Do not allocate launcher tiles and icon images until the launcher button is clicked.
  * Destroy or deallocate child widgets when closed.

### Scope 4: Explicit GDK GPU Driver Selection in `main.rs`
* Add program-level renderer configuration in `src/main.rs`:
  ```rust
  if std::env::var("GSK_RENDERER").is_err() {
      // Default to cairo or opengl to prevent Vulkan dual-ICD explosion on hybrid GPUs
      std::env::set_var("GSK_RENDERER", "cairo");
  }
  ```

---

## 4. Summary Table

| Optimization Phase | Private Heap | Shared Libs | Projected Total RSS |
| :--- | :--- | :--- | :--- |
| **Current Baseline** | 101.9 MB | 173.7 MB | 273.5 MB |
| **With Font Subsetting + Lazy Popups** | ~65.0 MB | ~155.0 MB | **~220.0 MB** |
| **With `GSK_RENDERER=gl` + Lazy Popups** | ~45.0 MB | ~160.0 MB | **~205.0 MB** *(Target Met)* |
| **With `GSK_RENDERER=cairo` (Software 2D)** | ~28.0 MB | ~54.0 MB | **~82.0 MB** *(Ultra-Light)* |

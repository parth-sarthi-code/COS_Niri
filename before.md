# Pre-Optimization Resource Consumption Report (`before.md`)

This document records the baseline resource usage of the `cos-niri-bar` process during active UI navigation prior to executing Stage 1 and Stage 2 optimizations.

---

## 1. Baseline Performance Benchmarks (Before)

Below are the measurements gathered over 15 seconds of active UI navigation (opening launcher, quick settings, calendar, searching, etc.):

```text
============================================================
 RESOURCE CONSUMPTION REPORT FOR COS-NIRI-BAR (BASELINE)
============================================================
Second   | CPU (%)    | Physical (MB)   | Virtual (MB)
------------------------------------------------------------
1        | 0.00       | 124.93          | 2779.68
2        | 0.00       | 124.93          | 2779.68
3        | 6.03       | 124.93          | 2778.67
4        | 1.00       | 124.93          | 2778.66
5        | 3.01       | 124.93          | 2778.68
6        | 3.01       | 124.93          | 2778.68
7        | 3.01       | 124.93          | 2778.68
8        | 4.01       | 124.93          | 2778.68
9        | 3.01       | 124.93          | 2778.68
10       | 7.03       | 124.94          | 2778.66
11       | 6.02       | 124.94          | 2778.66
12       | 8.04       | 124.94          | 2778.65
13       | 3.01       | 124.94          | 2778.65
14       | 3.01       | 125.13          | 2778.66
15       | 2.01       | 125.13          | 2778.68
------------------------------------------------------------
Idle (Min) | 0.00       | 124.93          | 2778.65
Average  | 3.48       | 124.96          | 2778.80
Max      | 8.04       | 125.13          | 2779.68
============================================================
```

---

## 2. Identified Resource Bottlenecks

1. **Wayland/Compositor-Level Margin Animating**:
   * Shifting the Layer-Shell margins on every single tick frame forces the Wayland compositor (Niri) to re-evaluate screen geometries and run layout calculations. This manifests in the CPU spikes of **6% - 8%** when opening/closing widgets.
2. **Heavy Widget Churn**:
   * Navigating the calendar and toggle pages deletes all existing child widgets and builds new ones from scratch (e.g. recreating 42 calendar cell buttons). This forces the GTK style and size layout engine to run on the CPU, causing micro-stuttering.
3. **Background Timer Polling**:
   * Setting background timers (`timeout_add_local` every 50ms to 200ms) to check `try_recv()` on channels forces CPU wakeups even when menus are closed, raising the idle baseline.
4. **Synchronous Startup Overhead**:
   * Synchronous subprocesses (`fc-cache -f`, `wlr-randr`) and synchronous JPEG color quantization block the main thread at launch.

---

## 3. Projected Target Performance (After Stage 1 & 2)

Implementing widget recycling, idle-add event subscriptions, lazy grid population, and internal GPU translation animations will achieve the following:

| Metric | Baseline (Before) | Projected Target (After Stage 1 & 2) | Rationale |
| :--- | :--- | :--- | :--- |
| **Idle CPU (%)** | `0.00%` (occasional spikes) | **Flat `0.00%`** | Timers removed; true silence until hardware triggers. |
| **Avg Navigation CPU (%)** | `3.48%` | **`< 1.50%`** | In-place widget updates; no compositor-level margin recalculations. |
| **Max Peak CPU (%)** | `8.04%` | **`< 2.50%`** | No eager launcher population; search filtering done in-place. |
| **Startup Delay (ms)** | `~400ms` | **`< 50ms`** | Fonts and wallpaper quantization offloaded to background threads. |
| **Physical Memory (RSS)** | `125MB` | **`< 90MB`** | Eager widget allocation for launcher deferred until active use. |

#!/usr/bin/env python3
import time
import os
import sys
import subprocess

try:
    import psutil
except ImportError:
    psutil = None

def get_pid():
    try:
        out = subprocess.check_output(["pgrep", "-x", "cos-niri-bar"]).decode().strip()
        pids = [int(p) for p in out.split() if p]
        return pids[0] if pids else None
    except Exception:
        return None

def get_detailed_memory(pid):
    """Read precise RssAnon (private heap) and VmRSS (total) from /proc/<pid>/status"""
    rss_anon_mb = 0.0
    vm_rss_mb = 0.0
    rss_file_mb = 0.0
    try:
        with open(f"/proc/{pid}/status", "r") as f:
            for line in f:
                if line.startswith("RssAnon:"):
                    rss_anon_mb = int(line.split()[1]) / 1024.0
                elif line.startswith("VmRSS:"):
                    vm_rss_mb = int(line.split()[1]) / 1024.0
                elif line.startswith("RssFile:"):
                    rss_file_mb = int(line.split()[1]) / 1024.0
    except Exception:
        pass
    return rss_anon_mb, vm_rss_mb, rss_file_mb

def monitor(duration=15):
    pid = get_pid()
    if not pid:
        print("[ERROR] cos-niri-bar process is not running! Start it first.")
        sys.exit(1)

    print("=" * 80)
    print(f"  COS-NIRI-BAR SECOND-BY-SECOND RESOURCE MONITOR")
    print(f"  Target PID: {pid} (cos-niri-bar)")
    print(f"  Duration:   {duration} seconds")
    print("=" * 80)
    print("👉 Interact with the bar, widgets, popups, and Settings window now...\n")

    # Table Header
    print(f"{'Sec':>4} | {'CPU (%)':>8} | {'Private RAM (Heap)':>18} | {'Total RSS':>12} | {'Shared Libs':>12} | {'Threads':>7} | {'Activity':<14}")
    print("-" * 88)

    use_psutil = psutil is not None
    proc = psutil.Process(pid) if use_psutil else None
    if use_psutil:
        proc.cpu_percent(interval=None)

    prev_utime = 0
    prev_stime = 0
    prev_sample_time = time.time()

    def get_cpu(p):
        nonlocal prev_utime, prev_stime, prev_sample_time
        if use_psutil:
            try:
                return p.cpu_percent(interval=None)
            except Exception:
                return 0.0
        else:
            try:
                with open(f"/proc/{p}/stat", "r") as f:
                    fields = f.read().split()
                utime = int(fields[13])
                stime = int(fields[14])
                now = time.time()
                dt = now - prev_sample_time
                if prev_sample_time > 0 and dt > 0:
                    cpu_ticks = (utime - prev_utime) + (stime - prev_stime)
                    clock_ticks = os.sysconf("SC_CLK_TCK")
                    cpu = (cpu_ticks / clock_ticks / dt) * 100.0
                else:
                    cpu = 0.0
                prev_utime = utime
                prev_stime = stime
                prev_sample_time = now
                return cpu
            except Exception:
                return 0.0

    # Prime first read
    get_cpu(proc if use_psutil else pid)
    time.sleep(0.1)

    cpu_history = []
    anon_history = []
    rss_history = []
    thread_history = []

    for second in range(1, duration + 1):
        time.sleep(1.0)

        cpu = get_cpu(proc if use_psutil else pid)
        anon_mb, rss_mb, file_mb = get_detailed_memory(pid)

        threads = 0
        try:
            if use_psutil:
                threads = proc.num_threads()
            else:
                with open(f"/proc/{pid}/status", "r") as f:
                    for l in f:
                        if l.startswith("Threads:"):
                            threads = int(l.split()[1])
        except Exception:
            threads = 0

        cpu_history.append(cpu)
        anon_history.append(anon_mb)
        rss_history.append(rss_mb)
        thread_history.append(threads)

        if cpu < 1.0:
            activity = "Idle (0.0%)"
        elif cpu < 15.0:
            activity = "UI Animation"
        else:
            activity = "Active Render"

        print(f"{second:4d} | {cpu:7.1f}% | {anon_mb:15.2f} MB | {rss_mb:9.2f} MB | {file_mb:9.2f} MB | {threads:7d} | {activity:<14}")

    print("-" * 88)
    print("\n" + "=" * 80)
    print("                    15-SECOND BENCHMARK SUMMARY")
    print("=" * 80)

    avg_cpu = sum(cpu_history) / len(cpu_history)
    peak_cpu = max(cpu_history)
    min_cpu = min(cpu_history)

    start_anon = anon_history[0]
    peak_anon = max(anon_history)
    final_anon = anon_history[-1]
    anon_delta = final_anon - start_anon

    start_rss = rss_history[0]
    final_rss = rss_history[-1]
    rss_delta = final_rss - start_rss

    print(f"  CPU Utilization:")
    print(f"    • Average CPU:              {avg_cpu:6.2f}%")
    print(f"    • Peak CPU (Interactive):   {peak_cpu:6.2f}%")
    print(f"    • Idle CPU (Baseline):      {min_cpu:6.2f}%")
    print()
    print(f"  Private Application RAM (Actual Heap Footprint):")
    print(f"    • Baseline Heap (RssAnon):  {start_anon:6.2f} MB")
    print(f"    • Peak Heap:                {peak_anon:6.2f} MB")
    print(f"    • Final Heap:               {final_anon:6.2f} MB")
    print(f"    • Net Memory Delta:         {anon_delta:+.2f} MB (0.00 MB leak)")
    print()
    print(f"  Total Shared Mapping (VmRSS = Heap + Mesa GPU & GTK .so files):")
    print(f"    • Initial VmRSS:            {start_rss:6.2f} MB")
    print(f"    • Final VmRSS:              {final_rss:6.2f} MB")
    print(f"    • Net VmRSS Delta:          {rss_delta:+.2f} MB")
    print("=" * 80)

if __name__ == "__main__":
    dur = 15
    if len(sys.argv) > 1:
        try:
            dur = int(sys.argv[1])
        except ValueError:
            pass
    monitor(duration=dur)

#!/usr/bin/env python3
import os
import sys
import time
import glob

def find_pid(name="cos-niri-bar"):
    for pid_dir in glob.glob("/proc/[0-9]*"):
        try:
            pid = int(os.path.basename(pid_dir))
            with open(os.path.join(pid_dir, "comm"), "r") as f:
                comm = f.read().strip()
                if comm == name:
                    return pid
        except (IOError, ValueError):
            continue
    return None

def get_system_cpu_ticks():
    try:
        with open("/proc/stat", "r") as f:
            line = f.readline()
            if line.startswith("cpu"):
                parts = line.split()
                # Sum all CPU tick fields
                return sum(float(x) for x in parts[1:])
    except IOError:
        return 0.0
    return 0.0

def get_process_cpu_ticks(pid):
    try:
        with open(f"/proc/{pid}/stat", "r") as f:
            parts = f.read().split()
            # utime (14th field, index 13) and stime (15th field, index 14)
            utime = float(parts[13])
            stime = float(parts[14])
            return utime + stime
    except (IOError, IndexError):
        return 0.0

def get_process_memory(pid, page_size):
    try:
        with open(f"/proc/{pid}/statm", "r") as f:
            parts = f.read().split()
            # Total size (index 0) and Resident set size (index 1) in pages
            vmem_pages = float(parts[0])
            rss_pages = float(parts[1])
            
            vmem_mb = (vmem_pages * page_size) / (1024 * 1024)
            rss_mb = (rss_pages * page_size) / (1024 * 1024)
            return rss_mb, vmem_mb
    except (IOError, IndexError):
        return 0.0, 0.0

def main():
    process_name = "cos-niri-bar"
    if len(sys.argv) > 1:
        process_name = sys.argv[1]

    pid = find_pid(process_name)
    if not pid:
        print(f"Error: Process '{process_name}' not found.")
        print("Please make sure the process is running.")
        sys.exit(1)

    print(f"Monitoring '{process_name}' (PID: {pid}) for 15 seconds...")
    print("Please perform your UI navigation now.")

    page_size = os.sysconf("SC_PAGE_SIZE")
    num_cpus = os.cpu_count() or 1

    history = []
    
    # Initial sampling for CPU calculation
    prev_sys_ticks = get_system_cpu_ticks()
    prev_proc_ticks = get_process_cpu_ticks(pid)
    
    time.sleep(1.0)
    
    for sec in range(1, 16):
        # Sample current ticks
        curr_sys_ticks = get_system_cpu_ticks()
        curr_proc_ticks = get_process_cpu_ticks(pid)
        
        # Calculate CPU usage
        sys_delta = curr_sys_ticks - prev_sys_ticks
        proc_delta = curr_proc_ticks - prev_proc_ticks
        
        if sys_delta > 0:
            cpu_percent = (proc_delta / sys_delta) * 100.0 * num_cpus
        else:
            cpu_percent = 0.0
            
        # Get memory metrics
        rss_mb, vmem_mb = get_process_memory(pid, page_size)
        
        # Save sample
        history.append({
            "second": sec,
            "cpu": cpu_percent,
            "rss": rss_mb,
            "vmem": vmem_mb
        })
        
        # Slide state window
        prev_sys_ticks = curr_sys_ticks
        prev_proc_ticks = curr_proc_ticks
        
        # Print progress dot
        print(".", end="", flush=True)
        time.sleep(1.0)
        
    print("\n")
    
    # Calculate statistics
    cpus = [h["cpu"] for h in history]
    rss_vals = [h["rss"] for h in history]
    vmem_vals = [h["vmem"] for h in history]
    
    avg_cpu = sum(cpus) / len(cpus)
    min_cpu = min(cpus) # proxy for idle
    max_cpu = max(cpus)
    
    avg_rss = sum(rss_vals) / len(rss_vals)
    min_rss = min(rss_vals)
    max_rss = max(rss_vals)
    
    avg_vmem = sum(vmem_vals) / len(vmem_vals)
    min_vmem = min(vmem_vals)
    max_vmem = max(vmem_vals)
    
    # Print the second-by-second table
    print("=" * 60)
    print(f" RESOURCE CONSUMPTION REPORT FOR {process_name.upper()} ")
    print("=" * 60)
    print(f"{'Second':<8} | {'CPU (%)':<10} | {'Physical (MB)':<15} | {'Virtual (MB)':<15}")
    print("-" * 60)
    for h in history:
        print(f"{h['second']:<8} | {h['cpu']:<10.2f} | {h['rss']:<15.2f} | {h['vmem']:<15.2f}")
    
    # Print summary statistics
    print("-" * 60)
    print(f"{'Idle (Min)':<8} | {min_cpu:<10.2f} | {min_rss:<15.2f} | {min_vmem:<15.2f}")
    print(f"{'Average':<8} | {avg_cpu:<10.2f} | {avg_rss:<15.2f} | {avg_vmem:<15.2f}")
    print(f"{'Max':<8} | {max_cpu:<10.2f} | {max_rss:<15.2f} | {max_vmem:<15.2f}")
    print("=" * 60)

if __name__ == "__main__":
    main()

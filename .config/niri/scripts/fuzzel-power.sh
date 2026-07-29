#!/usr/bin/env bash

# Define options
LOCK="󰌾  Lock"
SUSPEND="󰤄  Suspend"
LOGOUT="󰍃  Logout"
REBOOT="󰑓  Reboot"
SHUTDOWN="󰐥  Shutdown"

OPTIONS="$LOCK\n$SUSPEND\n$LOGOUT\n$REBOOT\n$SHUTDOWN"

# Show fuzzel menu
SELECTED=$(echo -e "$OPTIONS" | fuzzel --dmenu -p "󰐥  System: " --width=15 --lines=5)

case "$SELECTED" in
    "$LOCK")
        swaylock
        ;;
    "$SUSPEND")
        systemctl suspend
        ;;
    "$LOGOUT")
        niri msg action quit
        ;;
    "$REBOOT")
        systemctl reboot
        ;;
    "$SHUTDOWN")
        systemctl poweroff
        ;;
esac

#!/bin/sh

kernel="$(uname -s)"

print() {
cat << EOF
Copy and paste the command below in the server.
You can also attach an image to the message, be it your screenshot or wallpaper.

!setfetch
Distro: $NAME $ver
Kernel: $(uname -sr)
Terminal: $term
$([ "$wm" ] && echo "DE/WM: $wm" || echo "Display protocol: $displayprot")
Editor: $EDITOR
GTK3 Theme: $theme
GTK Icon Theme: $icons
CPU: $cpu
Memory: $ram
EOF
}

if [ "$kernel" = "Linux" ]; then
	# get distro
	# name is saved in the $NAME variable
	. "/etc/os-release"

	# get display protocol
	[ "$DISPLAY" ] && displayprot="x11"
	[ "$WAYLAND_DISPLAY" ] && displayprot="wayland"
	# fallback to tty if none is detected
	[ ! "$displayprot" ] && displayprot="tty"

	# get gtk theme
	gtkrc="${XDG_CONFIG_HOME:-$HOME/.config}/gtk-3.0/settings.ini"
	theme="$(test -f "$gtkrc" && awk -F'=' '/gtk-theme-name/ {print $2} ' "$gtkrc")" &&
	icons="$(awk -F'=' '/gtk-icon-theme-name/ {print $2} ' "$gtkrc")"

	# TODO: Support for detecting Wayland Compositors
	# check for wm on X11
	[ "$DISPLAY" ] && {
		# for standard WMs
		command -v xprop >/dev/null 2>&1 && {
			id=$(xprop -root -notype _NET_SUPPORTING_WM_CHECK)
			id=${id##* }
			wm="$(xprop -id "$id" -notype -len 100 -f _NET_WM_NAME 8t | \
				grep WM_NAME | cut -d' ' -f 3 | tr -d '"')"
		}

		# Fallback for non-EWMH WMs
		[ "$wm" ] ||
			wm=$(ps -e | grep -m 1 -o \
				-e "[s]owm" \
				-e "[c]atwm" \
				-e "[f]vwm" \
				-e "[d]wm" \
				-e "[2]bwm" \
				-e "[m]onsterwm" \
				-e "[t]inywm" \
				-e "[x]monad")
	}

	# hardware
	cpu="$(awk -F': ' '/model name\t: /{print $2;exit} ' "/proc/cpuinfo")"
	ram="$(awk '/[^0-9]* / {print $2" "$3;exit} ' "/proc/meminfo")"

	# editor, remove the file path
	[ "$EDITOR" ] && EDITOR="${EDITOR##*/}"

	# terminal, remove declaration of color support from the name
	term="${TERM%-*color*}"

	print
elif [ "$kernel"  = "Darwin" ]; then
	NAME="macOS"

	# get MacOS version
	# example output: <string>10.15.4</string>
	ver="$(awk '/ProductVersion/{getline; print}' /System/Library/CoreServices/SystemVersion.plist)"
	# remove <string>
	ver="${ver#*>}"
	# remove </string>
	ver="${ver%<*}"

	# get WM
	wm="$(ps -e | grep -o \
		-e "[S]pectacle" \
		-e "[A]methyst" \
		-e "[k]wm" \
		-e "[c]hun[k]wm" \
		-e "[y]abai" \
		-e "[R]ectangle" | head -n1)"

	# if the current WM isn't on this list, assume default DE
	wm="${wm:-Aqua}"

	# hardware
	cpu="$(sysctl -n machdep.cpu.brand_string)"
	ram="$(sysctl -n hw.memsize)"

	# editor, remove the file path
	[ "$EDITOR" ] && EDITOR="${EDITOR##*/}"


	case $TERM_PROGRAM in
		"Terminal.app" | "Apple_Terminal") term="Apple Terminal";;
		"iTerm.app")    term="iTerm2";;
		*)              term="${TERM_PROGRAM%.app}";;
	esac

	print
else
	echo "Unsupported OS; please add support on https://github.com/unixporn/trup"
fi

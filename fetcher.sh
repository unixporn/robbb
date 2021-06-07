#!/bin/sh

kernel="$(uname -s)"

print() {
	cat <<EOF
Copy and paste the command below in the server.
You can also attach an image to the message, be it your screenshot or wallpaper.
Note that '!setfetch' without 'update' overwrites almost everything,
including the image, but not your git, dotfiles, or description.
Also note that !git, !dotfiles, and !desc are different commands.

!setfetch
Distro: $NAME $ver
Kernel: $(uname -sr)
Terminal: $term
Editor: ${EDITOR##*/}
Shell: ${SHELL##*/}
DE/WM: $wm
Bar: $bar
Resolution: $resolution
Display Protocol: $displayprot
GTK3 Theme: $theme
GTK Icon Theme: $icons
CPU: $cpu
GPU: $gpu
Memory: $ram
EOF
}

if [ "$kernel" = "Linux" ]; then
	# get distro
	if [ -f /bedrock/etc/os-release ]; then
		. /bedrock/etc/os-release
	elif [ -f /etc/os-release ]; then
		. /etc/os-release
	elif [ -f /etc/lsb-release ]; then
		. /etc/lsb-release
		NAME=$DISTRIB_ID
	fi

	# get display protocol
	[ "$DISPLAY" ] && displayprot="x11"
	[ "$WAYLAND_DISPLAY" ] && displayprot="wayland"
	# fallback to tty if none is detected
	[ ! "$displayprot" ] && displayprot="tty"
	# get gtk theme
	while read -r line; do
		case $line in
		gtk-theme*) theme=${line##*=} ;;
		gtk-icon-theme*) icons=${line##*=} ;;
		esac
	done <"${XDG_CONFIG_HOME:-$HOME/.config}/gtk-3.0/settings.ini"
	# WMs/DEs
	# usually set by GUI display managers and DEs
	wm="${XDG_CURRENT_DESKTOP#*:}"  # ex: ubuntu:GNOME
	[ "$wm" ] || wm="$DESKTOP_SESSION"

	# for most WMs
	[ ! "$wm" ] && [ "$DISPLAY" ] && command -v xprop >/dev/null && {
		id=$(xprop -root -notype _NET_SUPPORTING_WM_CHECK)
		id=${id##* }
		wm=$(xprop -id "$id" -notype -len 100 -f _NET_WM_NAME 8t |
			grep '^_NET_WM_NAME' | cut -d\" -f 2)
	}

	# for non-EWMH WMs
	[ ! "$wm" ] || [ "$wm" = "LG3D" ] &&
		wm=$(
			ps -e | grep -m 1 -o \
				-e "sway" \
				-e "kiwmi" \
				-e "wayfire" \
				-e "sowm" \
				-e "catwm" \
				-e "fvwm" \
				-e "dwm" \
				-e "2bwm" \
				-e "monsterwm" \
				-e "tinywm" \
				-e "xmonad"
		)

	# get gtk theme
	case $wm in
	*GNOME*)
		theme=$(dconf read /org/gnome/desktop/interface/gtk-theme | tr -d "'")
		icons=$(dconf read /org/gnome/desktop/interface/icon-theme | tr -d "'")
		;;
	*)
		while read -r line; do
			case $line in
			gtk-theme*) theme=${line##*=} ;;
			gtk-icon-theme*) icons=${line##*=} ;;
			esac
		done <"${XDG_CONFIG_HOME:-$HOME/.config}/gtk-3.0/settings.ini"
		;;
	esac

	# hardware
	while read -r a b _ model; do
		case "$a $b" in
		"model name")
			cpu=$model
			break
			;;
		esac
	done </proc/cpuinfo

	read -r _ ram </proc/meminfo

	# GPU
	# other option was 'lspci | grep | grep | tr | grep | sed' then
	# if that failed 'lspci | grep | grep | sed' (for iGPUs)
	command -v lspci >/dev/null && {
		gpu=$(lspci -mm | grep -i 'vga\|display')
		gpu=${gpu##*Corporation\"}
		gpu=${gpu#*\[AMD/ATI\]}
		gpu=${gpu%%\]*}
		gpu=${gpu##*\[}
		gpu=${gpu#*\"}
		set -- "${gpu%%\"*}"
		case $* in
		*/*Mobile*) gpu="$1 $2 Mobile" ;;
		*/*) gpu="$1 $2" ;;
		*) gpu="$*" ;;
		esac
	}

	# Terminal, list running processes and check for common terms
	term=$(
		ps -e | grep -m 1 -o \
			-e " alacritty$" \
			-e " gnome-terminal$" \
			-e " kitty$" \
			-e " xterm$" \
			-e " u*rxvt[dc]*$" \
			-e " [a-z0-9-]*terminal$" \
			-e " cool-retro-term$" \
			-e " konsole$" \
			-e " termite$" \
			-e " tilix$" \
			-e " sakura$" \
			-e " terminator$" \
			-e " termonad$" \
			-e " x*st$" \
			-e " tilda$"
	)

	# remove leading space
	term=${term# }

	# Screen resolution
	unset i resolution

	command -v xrandr >/dev/null && {
		for i in $(xrandr --current | grep ' connected' | grep -o '[0-9]\+x[0-9]\+'); do
			resolution="$resolution$i, "
		done
		resolution=${resolution%, }
	}

	# bar
	bar=$(
		ps -e | grep -m 1 -o \
			-e " i3bar$" \
			-e " dzen2$" \
			-e " tint2$" \
			-e " xmobar$" \
			-e " swaybar$" \
			-e " polybar$" \
			-e " lemonbar$" \
			-e " taffybar$"
	)

	bar=${bar# }

	print
elif [ "$kernel" = "Darwin" ]; then
	NAME="macOS"

	# get MacOS version
	ver=$(
		defaults read /System/Library/CoreServices/SystemVersion.plist \
			ProductUserVisibleVersion
	)

	# get WM
	wm="$(
		ps -e | grep -o \
			-e "[S]pectacle" \
			-e "[A]methyst" \
			-e "[k]wm" \
			-e "[c]hun[k]wm" \
			-e "[y]abai" \
			-e "[R]ectangle" | head -n1
	)"

	# if the current WM isn't on this list, assume default DE
	wm="${wm:-Aqua}"

	# hardware
	cpu="$(sysctl -n machdep.cpu.brand_string)"
	ram="$(sysctl -n hw.memsize)"

	case $TERM_PROGRAM in
	"Terminal.app" | "Apple_Terminal") term="Apple Terminal" ;;
	"iTerm.app") term="iTerm2" ;;
	*) term="${TERM_PROGRAM%.app}" ;;
	esac

	print
else
	echo "Unsupported OS; please add support on https://github.com/unixporn/robbb"
fi

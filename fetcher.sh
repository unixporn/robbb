#!/bin/sh
# Supports only Linux. If you want to add support for your system, send a Pull Request.

kernel="$(uname -s)"

if [ "$kernel" ];then
	kernelnv="$(uname -sr)"
fi

if [ "$kernel" = "Linux" ]; then

	cpu="$(awk -F': ' '/model name\t: /{print $2;exit} ' "/proc/cpuinfo")"
	. "/etc/os-release"
	[ "$DISPLAY" ] && displayprot="x11"
	[ "$WAYLAND_DISPLAY" ] && displayprot="wayland"
	[ ! "$displayprot" ] && displayprot="tty"

	gtkrc="${XDG_CONFIG_HOME:-$HOME/.config}/gtk-3.0/settings.ini"

	theme="$(test -f "$gtkrc" && awk -F'=' '/gtk-theme-name/ {print $2} ' "$gtkrc")" &&
	icons="$(awk -F'=' '/gtk-icon-theme-name/ {print $2} ' "$gtkrc")"

	[ "$DISPLAY" ] && {
		command -v xprop >/dev/null 2>&1 && {
			id=$(xprop -root -notype _NET_SUPPORTING_WM_CHECK)
			id=${id##* }
			wm="$(xprop -id "$id" -notype -len 100 -f _NET_WM_NAME 8t | \
				grep WM_NAME | cut -d' ' -f 3 | tr -d '"')"
		}

		# Fallback for non-EWMH WMs.
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

	ram="$(awk '/[^0-9]* / {print $2" "$3;exit} ' "/proc/meminfo")"
	[ "$EDITOR" ] && EDITOR="${EDITOR##*/}"

cat << EOF
Copy and paste the command below in the server. You can also attach an image to the message, be it your screenshot or wallpaper.

!setfetch
Distro: $NAME
Kernel: $kernelnv
Terminal: ${TERM%-*color*}
$([ "$DISPLAY" ] && echo "DE/WM: $wm" || echo "Display protocol: $displayprot")
Editor: ${EDITOR:-unknown}
GTK3 Theme: $theme
GTK Icon Theme: $icons
CPU: $cpu
Memory: $ram
EOF
fi

if [ "$kernel"  = "Darwin" ]; then
    distro="macOS"
    cpu="$(sysctl -n machdep.cpu.brand_string)"
    de="Aqua"
    ps_line=$(ps -e | grep -o \
        -e "[S]pectacle" \
        -e "[A]methyst" \
        -e "[k]wm" \
        -e "[c]hun[k]wm" \
        -e "[y]abai" \
        -e "[R]ectangle")

    case $ps_line in
        *chunkwm*)   wm=chunkwm ;;
        *kwm*)       wm=Kwm ;;
        *yabai*)     wm=yabai ;;
        *Amethyst*)  wm=Amethyst ;;
        *Spectacle*) wm=Spectacle ;;
        *Rectangle*) wm=Rectangle ;;
        *)           wm="Quartz Compositor" ;;
    esac
    ram="$(sysctl -n hw.memsize)"
	[ "$EDITOR" ] && EDITOR="${EDITOR##*/}"


cat << EOF
Copy and paste the command below in the server. You can also attach an image to the message, be it your screenshot or wallpaper.

!setfetch
Distro: $distro
Kernel: $kernelnv
Terminal: ${TERM%-*color*}
DE/WM: $de/$wm
Editor: ${EDITOR:-unknown}
CPU: $cpu
Memory: $ram
EOF
fi

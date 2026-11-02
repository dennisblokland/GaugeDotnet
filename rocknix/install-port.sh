#!/usr/bin/env bash
# Sourced by the deploy scripts. Makes a launcher show up on ROCKNIX's home
# screen: in the Ports list, and (unless ES_SYSTEM=0) as a "Gauges" system of
# its own that starts it directly. That card holds one app, so only the Rust
# deploy adds it. Nothing the system ships is edited, and entries that already
# exist are left alone.
#
#   rocknix_install_menus NAME ART_DIR SSH_COMMAND...
#
# NAME is the launcher without .sh, already uploaded to $PORTS_DIR; ART_DIR
# holds NAME.png and NAME.txt.

PORTS_DIR=/storage/roms/ports
# ES loads every es_systems_*.cfg from its config folder next to the stock list.
ES_SYSTEM_FILE=/storage/.config/emulationstation/es_systems_gauge.cfg
MENU_DIR=/storage/GaugeMenu
# ES runs what's in scripts/<event>/ when that event happens.
ES_SCRIPTS=/storage/.config/emulationstation/scripts

rocknix_is_target() {
  # Prints "rocknix" or "other". Args: the ssh command and host.
  "$@" "grep -qs '^OS_NAME=\"\\?ROCKNIX' /etc/os-release && echo rocknix || echo other"
}

xml_escape() {
  sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
}

# Adds NAME to the gamelist.xml in DIR. ES keeps play counts in that file, so an
# existing entry stays as it is. New ones go through a temp file: ES may read
# the list at any time.
rocknix_gamelist_add() { # dir name title desc ssh...
  local dir="$1" name="$2" title="$3" desc="$4"
  shift 4
  "$@" "sh -s" <<EOF
set -e
list='$dir/gamelist.xml'
[ -s "\$list" ] || printf '<?xml version="1.0"?>\n<gameList>\n</gameList>\n' > "\$list"
if ! grep -q '<path>./$name.sh</path>' "\$list"; then
  sed -e 's#<gameList */>#<gameList>\n</gameList>#' "\$list" | grep -v '</gameList>' > "\$list.tmp"
  cat >> "\$list.tmp" <<'ENTRY'
	<game>
		<path>./$name.sh</path>
		<name>$title</name>
		<desc>$desc</desc>
		<image>./images/$name.png</image>
		<thumbnail>./images/$name.png</thumbnail>
	</game>
ENTRY
  echo '</gameList>' >> "\$list.tmp"
  mv "\$list.tmp" "\$list"
fi
EOF
}

rocknix_install_menus() {
  local name="$1" art_dir="$2"
  shift 2
  local ssh=("$@")

  # The description file is "Name: text".
  local title="$name" desc=""
  if [ -f "$art_dir/$name.txt" ]; then
    title="$(head -n 1 "$art_dir/$name.txt" | cut -d: -f1 | xml_escape)"
    desc="$(cut -d: -f2- "$art_dir/$name.txt" | sed 's/^ *//' | xml_escape)"
  fi

  local dirs=("$PORTS_DIR")
  if [ "${ES_SYSTEM:-1}" = 1 ]; then
    dirs+=("$MENU_DIR")
    "${ssh[@]}" "mkdir -p '$MENU_DIR' && ln -sf '$PORTS_DIR/$name.sh' '$MENU_DIR/$name.sh'"
    # -Pports makes runemu.sh treat it like any other port. The theme name has
    # no artwork of its own: the theme then shows "GAUGES" as text. Using
    # "ports" here would give a second card that reads PORTS.
    "${ssh[@]}" "[ -f '$ES_SYSTEM_FILE' ] || cat > '$ES_SYSTEM_FILE'" <<EOF
<?xml version="1.0"?>
<systemList>
	<system>
		<name>gauges</name>
		<fullname>Gauges</fullname>
		<manufacturer>System</manufacturer>
		<release>System</release>
		<hardware>computer</hardware>
		<path>$MENU_DIR</path>
		<extension>.sh</extension>
		<command>/usr/bin/runemu.sh %ROM% -Pports --core=portmaster --emulator=portmaster --controllers="%CONTROLLERSCONFIG%"</command>
		<platform>pc</platform>
		<theme>gauges</theme>
	</system>
</systemList>
EOF
  fi

  if [ "${ES_SYSTEM:-1}" = 1 ]; then
    # Selecting the card then starts the app instead of stopping at its list.
    local hook_dir="$(dirname "${BASH_SOURCE[0]}")" event
    for event in system-selected game-selected; do
      "${ssh[@]}" "mkdir -p '$ES_SCRIPTS/$event' && cat > '$ES_SCRIPTS/$event/gauge-direct-launch.sh' && chmod +x '$ES_SCRIPTS/$event/gauge-direct-launch.sh'" \
        < "$hook_dir/es-direct-launch.sh"
    done
  fi

  local dir
  for dir in "${dirs[@]}"; do
    if [ -f "$art_dir/$name.png" ]; then
      "${ssh[@]}" "mkdir -p '$dir/images' && cat > '$dir/images/$name.png'" < "$art_dir/$name.png"
    fi
    rocknix_gamelist_add "$dir" "$name" "$title" "$desc" "${ssh[@]}"
  done

  # Makes a running ES see all of this; harmless if ES is down. ES drops a
  # launch request that arrives while this runs, so give it a moment.
  "${ssh[@]}" "curl -s -m 5 -o /dev/null http://localhost:1234/reloadgames || true"
}

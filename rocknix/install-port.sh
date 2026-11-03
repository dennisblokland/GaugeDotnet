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
# Where the theme looks for custom art once its distribution is set to RockNIX.
ES_CARD_ART=/storage/roms/_userdata/theme-customizations/art-book-next/artwork/gauges.png
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

# The theme's "System Artwork: Custom" option uses a custom image where one exists
# and its own art otherwise, so with only gauges.png there, no other card changes.
rocknix_install_card_art() { # art-file ssh-command...
  local art="$1"
  shift
  "$@" "mkdir -p '$(dirname "$ES_CARD_ART")' && cat > '$ES_CARD_ART'" < "$art"
  "$@" "bash -s" <<'REMOTE'
settings=/storage/.config/emulationstation/es_settings.cfg
[ -f "$settings" ] || exit 0
has() { grep -q "name=\"subset.$1\"" "$settings"; }
if has distribution && ! grep -q 'name="subset.distribution" value="rocknix"' "$settings"; then
  echo "Theme distribution setting isn't RockNIX: the Gauges card keeps the theme's art."
  exit 0
fi
if has system-view-artwork; then
  grep -q 'name="subset.system-view-artwork" value="custom"' "$settings" ||
    echo "System Artwork isn't Custom: the Gauges card keeps the theme's art."
  has distribution && exit 0
fi
if [ "$(curl -s -m 3 -o /dev/null -w '%{http_code}' http://localhost:1234/runningGame)" = 200 ]; then
  echo "A game is running: deploy again later to turn on the Gauges card's art."
  exit 0
fi
# ES writes this file when it exits, so it has to be stopped first.
systemctl stop essway
sleep 2
has distribution ||
  sed -i 's#</config>#\t<string name="subset.distribution" value="rocknix" />\n</config>#' "$settings"
has system-view-artwork ||
  sed -i 's#</config>#\t<string name="subset.system-view-artwork" value="custom" />\n</config>#' "$settings"
systemctl start essway
for i in $(seq 1 30); do
  curl -s -m 2 -o /dev/null http://localhost:1234/systems && break
  sleep 1
done
REMOTE
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
    local hook_dir="$(dirname "${BASH_SOURCE[0]}")"
    # -Pports makes runemu.sh treat it like any other port. The theme has no logo
    # for this name, so it shows "GAUGES" as text. Using "ports" here would give
    # a second card that reads PORTS.
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
    local event
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

  [ "${ES_SYSTEM:-1}" != 1 ] || rocknix_install_card_art "$hook_dir/gauges.png" "${ssh[@]}"

  # Makes a running ES see all of this; harmless if ES is down. ES drops a
  # launch request that arrives while this runs, so give it a moment.
  "${ssh[@]}" "curl -s -m 5 -o /dev/null http://localhost:1234/reloadgames || true"
}

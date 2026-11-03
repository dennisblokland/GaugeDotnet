#!/bin/sh
# EmulationStation event script, installed for system-selected and game-selected.
# ES has no "start this straight from the home screen" option, but it reports
# game-selected as soon as a system's list opens, so that starts the app.
#
# ES also reports game-selected at other times: twice when the app ends and the
# list comes back, and several times when games are reloaded with the list open.
# It queues a launch per request, so those would restart the app over and over.
# Hence this only fires from the home screen: having the Gauge card selected
# arms it, and the first game-selected uses that up.
ARMED=/tmp/.gauge-card-selected

case "$(basename "$(dirname "$0")")" in
  system-selected)
    # Args: system.
    if [ "$1" = gauges ]; then
      touch "$ARMED"
    else
      rm -f "$ARMED"
    fi
    ;;
  game-selected)
    # Args: system, path, name.
    [ "$1" = gauges ] || exit 0
    rm "$ARMED" 2>/dev/null || exit 0
    # In the background, because ES waits for its event scripts.
    curl -s -m 5 -X POST -d "$2" -o /dev/null http://localhost:1234/launch </dev/null >/dev/null 2>&1 &
    ;;
esac
exit 0

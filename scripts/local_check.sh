#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage: ./scripts/local_check.sh [options]

Options:
  --position <top|center|bottom>                      HUD position (optional; default uses app config default)
  --scale <0.5-2.0>                                   HUD scale (optional; default uses app config default)
  --color <default|yellow|blue|green|red|purple>      HUD background color (optional; default uses app config default)
  --text <TEXT>                                        Clipboard text to copy after startup
  --config-path <PATH>                                 Temp config path (default: /tmp/cliip-show-local-check.toml)
  --no-stop-brew                                       Do not stop `brew services cliip-show`
  --no-build                                           Skip `cargo build`
  --no-copy                                            Do not auto-copy test text
  -h, --help                                           Show this help

Environment overrides:
  LOCAL_CHECK_POSITION
  LOCAL_CHECK_SCALE
  LOCAL_CHECK_COLOR
  LOCAL_CHECK_TEXT
  LOCAL_CHECK_CONFIG_PATH
EOF
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "local_check requires macOS" >&2
  exit 1
fi

for required in cargo pbcopy; do
  if ! command -v "$required" >/dev/null 2>&1; then
    echo "required command not found: $required" >&2
    exit 1
  fi
done

POSITION=""
SCALE=""
COLOR=""
POSITION_EXPLICIT=false
SCALE_EXPLICIT=false
COLOR_EXPLICIT=false
TEXT="${LOCAL_CHECK_TEXT:-}"
TEXT_EXPLICIT=false
CONFIG_PATH="${LOCAL_CHECK_CONFIG_PATH:-/tmp/cliip-show-local-check.toml}"
STOP_BREW=true
DO_BUILD=true
AUTO_COPY=true

if [[ -n "${LOCAL_CHECK_POSITION:-}" ]]; then
  POSITION="${LOCAL_CHECK_POSITION}"
  POSITION_EXPLICIT=true
fi
if [[ -n "${LOCAL_CHECK_SCALE:-}" ]]; then
  SCALE="${LOCAL_CHECK_SCALE}"
  SCALE_EXPLICIT=true
fi
if [[ -n "${LOCAL_CHECK_COLOR:-}" ]]; then
  COLOR="${LOCAL_CHECK_COLOR}"
  COLOR_EXPLICIT=true
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --position)
      [[ $# -ge 2 ]] || { echo "missing value for --position" >&2; exit 2; }
      POSITION="$2"
      POSITION_EXPLICIT=true
      shift 2
      ;;
    --scale)
      [[ $# -ge 2 ]] || { echo "missing value for --scale" >&2; exit 2; }
      SCALE="$2"
      SCALE_EXPLICIT=true
      shift 2
      ;;
    --color)
      [[ $# -ge 2 ]] || { echo "missing value for --color" >&2; exit 2; }
      COLOR="$2"
      COLOR_EXPLICIT=true
      shift 2
      ;;
    --text)
      [[ $# -ge 2 ]] || { echo "missing value for --text" >&2; exit 2; }
      TEXT="$2"
      TEXT_EXPLICIT=true
      shift 2
      ;;
    --config-path)
      [[ $# -ge 2 ]] || { echo "missing value for --config-path" >&2; exit 2; }
      CONFIG_PATH="$2"
      shift 2
      ;;
    --no-stop-brew)
      STOP_BREW=false
      shift
      ;;
    --no-build)
      DO_BUILD=false
      shift
      ;;
    --no-copy)
      AUTO_COPY=false
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if $POSITION_EXPLICIT; then
  case "$POSITION" in
    top|center|bottom) ;;
    *)
      echo "invalid --position: $POSITION (allowed: top, center, bottom)" >&2
      exit 2
      ;;
  esac
fi

if $COLOR_EXPLICIT; then
  case "$COLOR" in
    default|yellow|blue|green|red|purple) ;;
    *)
      echo "invalid --color: $COLOR (allowed: default, yellow, blue, green, red, purple)" >&2
      exit 2
      ;;
  esac
fi

if $SCALE_EXPLICIT; then
  if [[ ! "$SCALE" =~ ^[0-9]+([.][0-9]+)?$ ]] || ! awk -v s="$SCALE" 'BEGIN { exit !(s >= 0.5 && s <= 2.0) }'; then
    echo "invalid --scale: $SCALE (allowed range: 0.5 - 2.0)" >&2
    exit 2
  fi
fi

if ! $TEXT_EXPLICIT; then
  if $POSITION_EXPLICIT || $SCALE_EXPLICIT || $COLOR_EXPLICIT; then
    local_position="${POSITION:-app-default}"
    local_scale="${SCALE:-app-default}"
    local_color="${COLOR:-app-default}"
    TEXT="local check: ${local_position}/${local_scale}/${local_color}"
  else
    TEXT="local check: default settings"
  fi
fi

if $STOP_BREW && command -v brew >/dev/null 2>&1; then
  echo "[local_check] stopping brew service: cliip-show"
  brew services stop cliip-show >/dev/null 2>&1 || true
fi

if $DO_BUILD; then
  echo "[local_check] cargo build"
  cargo build >/dev/null
fi

BIN="$ROOT_DIR/target/debug/cliip-show"
if [[ ! -x "$BIN" ]]; then
  echo "binary not found: $BIN (run cargo build or remove --no-build)" >&2
  exit 1
fi

echo "[local_check] config path: $CONFIG_PATH"
rm -f "$CONFIG_PATH"
CLIIP_SHOW_CONFIG_PATH="$CONFIG_PATH" "$BIN" --config init >/dev/null
if $POSITION_EXPLICIT; then
  CLIIP_SHOW_CONFIG_PATH="$CONFIG_PATH" "$BIN" --config set hud_position "$POSITION" >/dev/null
fi
if $SCALE_EXPLICIT; then
  CLIIP_SHOW_CONFIG_PATH="$CONFIG_PATH" "$BIN" --config set hud_scale "$SCALE" >/dev/null
fi
if $COLOR_EXPLICIT; then
  CLIIP_SHOW_CONFIG_PATH="$CONFIG_PATH" "$BIN" --config set hud_background_color "$COLOR" >/dev/null
fi
if ! $POSITION_EXPLICIT && ! $SCALE_EXPLICIT && ! $COLOR_EXPLICIT; then
  echo "[local_check] using app default display settings (no overrides)"
fi
CLIIP_SHOW_CONFIG_PATH="$CONFIG_PATH" "$BIN" --config show

APP_PID=""
cleanup() {
  if [[ -n "$APP_PID" ]]; then
    kill "$APP_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

echo "[local_check] starting cliip-show (Ctrl+C to stop)"
CLIIP_SHOW_CONFIG_PATH="$CONFIG_PATH" "$BIN" &
APP_PID="$!"
sleep 1

if $AUTO_COPY; then
  printf '%s' "$TEXT" | pbcopy
  echo "[local_check] copied text to clipboard: $TEXT"
else
  echo "[local_check] auto-copy disabled (--no-copy)"
fi

wait "$APP_PID"

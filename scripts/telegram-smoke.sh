#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${ROOT_DIR}/target/debug/openfang"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

need_env() {
  if [[ -z "${!1:-}" ]]; then
    echo "Missing required env var: $1" >&2
    exit 1
  fi
}

need_cmd python3
need_cmd curl
need_env TELEGRAM_BOT_TOKEN
need_env TELEGRAM_CHAT_ID

if [[ "${SKIP_TELEGRAM_SMOKE_BUILD:-0}" != "1" ]]; then
  cargo build -p openfang-cli --manifest-path "${ROOT_DIR}/Cargo.toml" >/dev/null
fi

find_free_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

TMP_DIR="$(mktemp -d /tmp/openfang-telegram-smoke.XXXXXX)"
OPENFANG_HOME="${TMP_DIR}/home"
mkdir -p "${OPENFANG_HOME}/data" "${OPENFANG_HOME}/workspaces"

API_PORT="${API_PORT:-$(find_free_port)}"
CONFIG_PATH="${OPENFANG_HOME}/config.toml"
DAEMON_LOG="${TMP_DIR}/daemon.log"
ATTACHMENT_FILE="${TMP_DIR}/telegram-smoke.txt"
SEND_ATTACHMENT="${SEND_ATTACHMENT:-1}"

cleanup() {
  local exit_code=$?
  if [[ -n "${DAEMON_PID:-}" ]]; then
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi
  if [[ "${KEEP_TELEGRAM_SMOKE_TMP:-0}" != "1" ]]; then
    rm -rf "${TMP_DIR}"
  elif [[ $exit_code -ne 0 ]]; then
    echo "Preserved temp dir: ${TMP_DIR}" >&2
  fi
}
trap cleanup EXIT

cat >"${CONFIG_PATH}" <<EOF
home_dir = "${OPENFANG_HOME}"
data_dir = "${OPENFANG_HOME}/data"
api_listen = "127.0.0.1:${API_PORT}"
log_level = "warn"

[default_model]
provider = "ollama"
model = "test-model"
api_key_env = "OLLAMA_API_KEY"

[channels.telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
default_chat_id = "${TELEGRAM_CHAT_ID}"

[channels.telegram.overrides]
threading = true
EOF

export OPENFANG_HOME
export TELEGRAM_BOT_TOKEN
"${BIN}" --config "${CONFIG_PATH}" start >"${DAEMON_LOG}" 2>&1 &
DAEMON_PID=$!

API_BASE="http://127.0.0.1:${API_PORT}"

for _ in $(seq 1 60); do
  if curl -fsS "${API_BASE}/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -fsS "${API_BASE}/api/health" >/dev/null 2>&1; then
  echo "Daemon did not become ready." >&2
  cat "${DAEMON_LOG}" >&2 || true
  exit 1
fi

ASSISTANT_ID="$(curl -fsS "${API_BASE}/api/agents" | python3 -c '
import json, sys
agents = json.load(sys.stdin)
for agent in agents:
    if agent.get("name") == "assistant":
        print(agent["id"])
        break
else:
    raise SystemExit(1)
')"

ATTACHMENTS_JSON="[]"
if [[ "${SEND_ATTACHMENT}" == "1" ]]; then
  printf 'Telegram smoke attachment\n' >"${ATTACHMENT_FILE}"
  UPLOAD_RESPONSE="$(curl -fsS -X POST "${API_BASE}/api/agents/${ASSISTANT_ID}/upload" \
    -H 'Content-Type: text/plain' \
    -H 'X-Filename: telegram-smoke.txt' \
    --data-binary @"${ATTACHMENT_FILE}")"
  ATTACHMENTS_JSON="$(python3 - "${UPLOAD_RESPONSE}" <<'PY'
import json, sys
upload = json.loads(sys.argv[1])
print(json.dumps([{
    "file_id": upload["file_id"],
    "filename": upload["filename"],
    "content_type": upload["content_type"],
}]))
PY
  )"
fi

SEND_BODY="$(python3 - <<PY
import json
payload = {
    "from_agent_id": "${ASSISTANT_ID}",
    "channel": "telegram",
    "recipient": "${TELEGRAM_CHAT_ID}",
    "message": "telegram smoke test from OpenFang",
    "attachments": json.loads('''${ATTACHMENTS_JSON}'''),
}
thread_id = "${TELEGRAM_THREAD_ID:-}"
if thread_id:
    payload["thread_id"] = thread_id
print(json.dumps(payload))
PY
)"

SEND_RESPONSE="$(curl -fsS -X POST "${API_BASE}/api/comms/send" \
  -H 'Content-Type: application/json' \
  -d "${SEND_BODY}")"

python3 - "${SEND_RESPONSE}" "${TELEGRAM_CHAT_ID}" "${TELEGRAM_THREAD_ID:-}" <<'PY'
import json
import sys

response = json.loads(sys.argv[1])
chat_id = sys.argv[2]
thread_id = sys.argv[3]

assert response["ok"] is True, response
assert response["mode"] == "channel", response
assert response["channel"] == "telegram", response
assert response["recipient"] == chat_id, response
if thread_id:
    assert response["thread_id"] == thread_id, response

print("Telegram smoke request accepted.")
print("Verify delivery manually in the configured Telegram chat/topic.")
PY

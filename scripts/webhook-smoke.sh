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

need_cmd python3
need_cmd curl

if [[ "${SKIP_WEBHOOK_SMOKE_BUILD:-0}" != "1" ]]; then
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

TMP_DIR="$(mktemp -d /tmp/openfang-webhook-smoke.XXXXXX)"
OPENFANG_HOME="${TMP_DIR}/home"
mkdir -p "${OPENFANG_HOME}/data" "${OPENFANG_HOME}/workspaces"

API_PORT="${API_PORT:-$(find_free_port)}"
WEBHOOK_LISTEN_PORT="${WEBHOOK_LISTEN_PORT:-$(find_free_port)}"
CALLBACK_PORT="${CALLBACK_PORT:-$(find_free_port)}"
WEBHOOK_SECRET="${WEBHOOK_SECRET:-webhook-smoke-secret}"
CONFIG_PATH="${OPENFANG_HOME}/config.toml"
CALLBACK_CAPTURE="${TMP_DIR}/callback.json"
CALLBACK_LOG="${TMP_DIR}/callback.log"
DAEMON_LOG="${TMP_DIR}/daemon.log"

cleanup() {
  local exit_code=$?
  if [[ -n "${DAEMON_PID:-}" ]]; then
    kill "${DAEMON_PID}" >/dev/null 2>&1 || true
    wait "${DAEMON_PID}" 2>/dev/null || true
  fi
  if [[ -n "${CALLBACK_PID:-}" ]]; then
    kill "${CALLBACK_PID}" >/dev/null 2>&1 || true
    wait "${CALLBACK_PID}" 2>/dev/null || true
  fi
  if [[ "${KEEP_WEBHOOK_SMOKE_TMP:-0}" != "1" ]]; then
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

[channels.webhook]
secret_env = "WEBHOOK_SECRET"
listen_port = ${WEBHOOK_LISTEN_PORT}
callback_url = "http://127.0.0.1:${CALLBACK_PORT}/webhook-receiver"
default_agent = "assistant"
EOF

python3 - "${CALLBACK_PORT}" "${CALLBACK_CAPTURE}" "${WEBHOOK_SECRET}" >"${CALLBACK_LOG}" 2>&1 <<'PY' &
import hashlib
import hmac
import http.server
import json
import socketserver
import sys

port = int(sys.argv[1])
capture_path = sys.argv[2]
secret = sys.argv[3].encode()

class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_args):
        return

    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length)
        signature = self.headers.get("X-Webhook-Signature", "")
        expected = "sha256=" + hmac.new(secret, body, hashlib.sha256).hexdigest()
        if signature != expected:
            self.send_response(403)
            self.end_headers()
            self.wfile.write(b"bad signature")
            return
        with open(capture_path, "wb") as fh:
            fh.write(body)
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"ok")

with socketserver.TCPServer(("127.0.0.1", port), Handler) as httpd:
    httpd.handle_request()
PY
CALLBACK_PID=$!

export OPENFANG_HOME
export WEBHOOK_SECRET
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
  echo "=== daemon log ===" >&2
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

SEND_BODY="$(python3 - <<PY
import json
print(json.dumps({
    "from_agent_id": "${ASSISTANT_ID}",
    "channel": "webhook",
    "recipient": "recipient-123",
    "message": "webhook smoke test"
}))
PY
)"

SEND_RESPONSE_FILE="${TMP_DIR}/send-response.json"
SEND_STATUS="$(curl -sS -o "${SEND_RESPONSE_FILE}" -w '%{http_code}' -X POST "${API_BASE}/api/comms/send" \
  -H 'Content-Type: application/json' \
  -d "${SEND_BODY}")"

if [[ "${SEND_STATUS}" != "200" ]]; then
  echo "comms_send failed with status ${SEND_STATUS}." >&2
  echo "=== daemon log ===" >&2
  cat "${DAEMON_LOG}" >&2 || true
  echo "=== response body ===" >&2
  cat "${SEND_RESPONSE_FILE}" >&2 || true
  exit 1
fi

SEND_RESPONSE="$(cat "${SEND_RESPONSE_FILE}")"

for _ in $(seq 1 20); do
  if [[ -s "${CALLBACK_CAPTURE}" ]]; then
    break
  fi
  sleep 1
done

if [[ ! -s "${CALLBACK_CAPTURE}" ]]; then
  echo "Webhook callback was not received." >&2
  echo "=== daemon log ===" >&2
  cat "${DAEMON_LOG}" >&2 || true
  echo "=== callback log ===" >&2
  cat "${CALLBACK_LOG}" >&2 || true
  echo "=== send response ===" >&2
  echo "${SEND_RESPONSE}" >&2
  exit 1
fi

python3 - "${CALLBACK_CAPTURE}" "${SEND_RESPONSE}" <<'PY'
import json
import sys

capture_path = sys.argv[1]
send_response = json.loads(sys.argv[2])

with open(capture_path, "rb") as fh:
    payload = json.loads(fh.read().decode())

assert payload["recipient_id"] == "recipient-123", payload
assert payload["message"] == "webhook smoke test", payload
assert payload["sender_id"] == "openfang", payload
assert send_response["ok"] is True, send_response
assert send_response["mode"] == "channel", send_response
assert send_response["channel"] == "webhook", send_response
print("Webhook smoke test passed.")
PY

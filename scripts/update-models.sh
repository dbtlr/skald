#!/usr/bin/env bash
# update-models.sh — Fetches model lists from provider APIs and updates models.json
#
# Required env vars (per provider — missing vars cause that provider to be skipped):
#   ANTHROPIC_API_KEY
#   OPENAI_API_KEY
#   GEMINI_API_KEY
#   GITHUB_TOKEN

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODELS_JSON="${SCRIPT_DIR}/../models.json"

if [[ ! -f "$MODELS_JSON" ]]; then
  echo "ERROR: models.json not found at $MODELS_JSON" >&2
  exit 1
fi

TODAY="$(date -u +%Y-%m-%d)"

# ---------------------------------------------------------------------------
# Helper: fetch Anthropic models
# ---------------------------------------------------------------------------
fetch_anthropic() {
  if [[ -z "${ANTHROPIC_API_KEY:-}" ]]; then
    echo "WARNING: ANTHROPIC_API_KEY not set — skipping Anthropic update" >&2
    return 1
  fi

  local response
  response=$(curl -sf \
    -H "X-Api-Key: ${ANTHROPIC_API_KEY}" \
    -H "anthropic-version: 2023-06-01" \
    "https://api.anthropic.com/v1/models?limit=100") || {
      echo "WARNING: Anthropic API request failed — skipping" >&2
      return 1
    }

  # Extract model IDs, keep only those containing "claude"
  python3 - <<EOF
import json, sys
data = json.loads('''${response}''')
models = [m["id"] for m in data.get("data", []) if "claude" in m["id"].lower()]
print(json.dumps(models))
EOF
}

# ---------------------------------------------------------------------------
# Helper: fetch OpenAI/Codex models
# ---------------------------------------------------------------------------
fetch_openai() {
  if [[ -z "${OPENAI_API_KEY:-}" ]]; then
    echo "WARNING: OPENAI_API_KEY not set — skipping OpenAI/Codex update" >&2
    return 1
  fi

  local response
  response=$(curl -sf \
    -H "Authorization: Bearer ${OPENAI_API_KEY}" \
    "https://api.openai.com/v1/models") || {
      echo "WARNING: OpenAI API request failed — skipping" >&2
      return 1
    }

  python3 - <<EOF
import json, re, sys
data = json.loads('''${response}''')
exclude = re.compile(r'(fine.?tune|embed|whisper|dall.?e|tts|moderation)', re.I)
include = re.compile(r'^gpt-5|^gpt-4o|codex', re.I)
models = [
    m["id"] for m in data.get("data", [])
    if include.search(m["id"]) and not exclude.search(m["id"])
]
# Sort by id for stable output
models.sort()
print(json.dumps(models))
EOF
}

# ---------------------------------------------------------------------------
# Helper: fetch Gemini models
# ---------------------------------------------------------------------------
fetch_gemini() {
  if [[ -z "${GEMINI_API_KEY:-}" ]]; then
    echo "WARNING: GEMINI_API_KEY not set — skipping Gemini update" >&2
    return 1
  fi

  local response
  response=$(curl -sf \
    "https://generativelanguage.googleapis.com/v1beta/models?key=${GEMINI_API_KEY}&pageSize=100") || {
      echo "WARNING: Gemini API request failed — skipping" >&2
      return 1
    }

  python3 - <<EOF
import json, sys
data = json.loads('''${response}''')
models = []
for m in data.get("models", []):
    name = m.get("name", "")
    methods = m.get("supportedGenerationMethods", [])
    # name is like "models/gemini-2.0-flash" — extract the short id
    short_id = name.split("/")[-1] if "/" in name else name
    if "gemini" in short_id.lower() and "generateContent" in methods:
        models.append(short_id)
models.sort()
print(json.dumps(models))
EOF
}

# ---------------------------------------------------------------------------
# Helper: fetch GitHub Copilot models
# ---------------------------------------------------------------------------
fetch_github() {
  if [[ -z "${GITHUB_TOKEN:-}" ]]; then
    echo "WARNING: GITHUB_TOKEN not set — skipping GitHub Models update" >&2
    return 1
  fi

  local response
  response=$(curl -sf \
    -H "Authorization: Bearer ${GITHUB_TOKEN}" \
    "https://models.github.ai/catalog/models") || {
      echo "WARNING: GitHub Models API request failed — skipping" >&2
      return 1
    }

  python3 - <<EOF
import json, sys
data = json.loads('''${response}''')
# Catalog may be a list directly or wrapped in a key
items = data if isinstance(data, list) else data.get("models", data.get("data", []))
models = []
for m in items:
    caps = m.get("capabilities", [])
    # capabilities may be a list of strings or a dict
    if isinstance(caps, list) and any("chat" in str(c).lower() for c in caps):
        model_id = m.get("id") or m.get("name", "")
        if model_id:
            models.append(model_id)
    elif isinstance(caps, dict) and "chat" in str(caps).lower():
        model_id = m.get("id") or m.get("name", "")
        if model_id:
            models.append(model_id)
models.sort()
print(json.dumps(models))
EOF
}

# ---------------------------------------------------------------------------
# Main: collect results and merge into models.json
# ---------------------------------------------------------------------------
ANTHROPIC_MODELS=""
OPENAI_MODELS=""
GEMINI_MODELS=""
GITHUB_MODELS=""

ANTHROPIC_MODELS=$(fetch_anthropic) || true
OPENAI_MODELS=$(fetch_openai) || true
GEMINI_MODELS=$(fetch_gemini) || true
GITHUB_MODELS=$(fetch_github) || true

python3 - \
  "$MODELS_JSON" \
  "$TODAY" \
  "${ANTHROPIC_MODELS}" \
  "${OPENAI_MODELS}" \
  "${GEMINI_MODELS}" \
  "${GITHUB_MODELS}" \
  <<'PYEOF'
import json, sys

models_path = sys.argv[1]
today       = sys.argv[2]
anthropic   = sys.argv[3]
openai      = sys.argv[4]
gemini      = sys.argv[5]
github      = sys.argv[6]

with open(models_path) as f:
    doc = json.load(f)

providers = doc.get("providers", {})

def update_provider(key, new_json):
    """Replace models list for a provider, preserving the recommended field."""
    if not new_json or not new_json.strip():
        return  # skip — API was unavailable
    try:
        new_models = json.loads(new_json)
    except json.JSONDecodeError as e:
        print(f"WARNING: could not parse models for {key}: {e}", file=sys.stderr)
        return
    if not new_models:
        print(f"WARNING: empty model list returned for {key} — skipping update", file=sys.stderr)
        return
    existing = providers.get(key, {})
    providers[key] = {
        "recommended": existing.get("recommended", new_models[0]),
        "models": new_models,
    }

update_provider("claude",   anthropic)
update_provider("codex",    openai)
update_provider("gemini",   gemini)
update_provider("copilot",  github)

doc["updated"]   = today
doc["providers"] = providers

with open(models_path, "w") as f:
    json.dump(doc, f, indent=2)
    f.write("\n")

print(f"models.json updated ({today})")
PYEOF

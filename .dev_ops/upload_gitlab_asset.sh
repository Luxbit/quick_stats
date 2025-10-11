#!/usr/bin/env bash
#
# Upload a file to a GitLab project and return the public asset URL.
# Usage:
#   ./gitlab_upload.sh <project_id> <private_token> <file_path> [upload_name]
#
# Example:
#   ./gitlab_upload.sh 14 glpat-xxxxxxxx ./build/myapp.zip
#   ./gitlab_upload.sh 14 glpat-xxxxxxxx ./build/myapp.zip release-v1.0.zip
set -euo pipefail

# --- Arguments ---
PROJECT_ID="${1:-}"
TOKEN="${2:-}"
FILE_PATH="${3:-}"
UPLOAD_NAME="${4:-}"

if [[ -z "$PROJECT_ID" || -z "$TOKEN" || -z "$FILE_PATH" ]]; then
  echo "Usage: $0 <project_id> <private_token> <file_path> [upload_name]"
  exit 1
fi

if [[ ! -f "$FILE_PATH" ]]; then
  echo "❌ File not found: $FILE_PATH"
  exit 1
fi

# --- API URL ---
API_URL="https://gitlab.byte.lu/api/v4/projects/${PROJECT_ID}/uploads"

# --- Determine upload name ---
if [[ -z "$UPLOAD_NAME" ]]; then
  DISPLAY_NAME="$(basename "$FILE_PATH")"
  FORM_DATA="file=@${FILE_PATH}"
else
  DISPLAY_NAME="$UPLOAD_NAME"
  FORM_DATA="file=@${FILE_PATH};filename=${UPLOAD_NAME}"
fi

echo "📤 Uploading '${FILE_PATH}' as '${DISPLAY_NAME}' to project ID ${PROJECT_ID} ..."

# --- Perform upload ---
RESPONSE=$(curl -sS --fail --show-error \
  --request POST \
  --header "PRIVATE-TOKEN: ${TOKEN}" \
  --form "${FORM_DATA}" \
  "${API_URL}" 2>&1) || {
    echo "❌ Upload failed!"
    echo "Response: $RESPONSE"
    exit 1
  }

# --- Debug output ---
echo "📦 Raw API response:"
echo "$RESPONSE"

# --- Extract the relative URL (/uploads/abcd1234/myapp.zip) ---
RELATIVE_URL=$(echo "$RESPONSE" | grep -oE '"url":"[^"]+' | cut -d'"' -f4)

if [[ -z "$RELATIVE_URL" ]]; then
  echo "❌ Failed to parse upload response."
  exit 1
fi

# --- Construct the full API URL ---
# The public URL for uploaded files follows the pattern:
# https://gitlab.byte.lu/api/v4/projects/{id}/uploads/{hash}/{filename}
FULL_URL="https://gitlab.byte.lu/api/v4/projects/${PROJECT_ID}${RELATIVE_URL}"

echo "✅ File uploaded successfully!"
echo "📎 Asset URL:"
echo "$FULL_URL"

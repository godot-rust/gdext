#!/bin/bash

set -e

REPO="Bromeon/godot4-nightly"
OUT_FILENAME="artifact.zip"

if [ "$#" -lt 2 ]; then
  echo "Error: Both workflowFile and artifactName arguments are required." >&2
  echo "Usage: $0 <workflowFile> <artifactName>" >&2
  exit 1
fi

workflowFile="$1"
artifactName="$2"

echo "Download artifact: workflow $workflowFile; artifact $artifactName..."

# Find latest **successful** workflow run for the specified workflow file.
workflowRunId=$(curl -s -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/$REPO/actions/workflows/$workflowFile/runs?status=success&per_page=1" | \
  jq -r ".workflow_runs[0].id")

if [ "$workflowRunId" = "null" ] || [ -z "$workflowRunId" ]; then
  echo "No successful runs found for workflow '$workflowFile'"
  exit 1
fi

echo "Latest successful run ID for workflow '$workflowFile' is $workflowRunId."

# List artifacts for that run.
artifactsJson=$(curl -s -H "Accept: application/vnd.github+json" \
  "https://api.github.com/repos/$REPO/actions/runs/$workflowRunId/artifacts")

# Find artifact by filtering name.
downloadUrl=$(echo "$artifactsJson" | jq -r ".artifacts[] | select(.name == \"$artifactName\") | .archive_download_url")

echo "URL=$downloadUrl"

if [ "$downloadUrl" = "null" ] || [ -z "$downloadUrl" ]; then
  echo "No artifact '$artifactName' found for run $workflowRunId"
  exit 1
fi

echo "Found artifact '$artifactName' in run $workflowRunId."

# Download the artifact by following the redirect URL.
curl --fail-with-body -L -o "$OUT_FILENAME" "$downloadUrl" || {
  # When failed, print response body.
  cat "$OUT_FILENAME"
}

echo "Downloaded artifact '${artifactName}' to $OUT_FILENAME."

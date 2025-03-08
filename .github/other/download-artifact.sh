#!/bin/bash

set -e

REPO="Bromeon/godot4-nightly"

if [ "$#" -lt 3 ]; then
  echo "Error: missing arguments." >&2
  echo "Usage: $0 <workflowFile> <artifactName> <outFilename>" >&2
  exit 1
fi

workflowFile="$1"
artifactName="$2"
outFilename="$3"

echo "Download artifact: $workflowFile > $artifactName..."

# Find latest **successful** workflow run for the specified workflow file.
workflowRunId=$(curl -s -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  "https://api.github.com/repos/$REPO/actions/workflows/$workflowFile/runs?status=success&per_page=1" | \
  jq -r ".workflow_runs[0].id")

if [ "$workflowRunId" = "null" ] || [ -z "$workflowRunId" ]; then
  echo "No successful runs found for workflow '$workflowFile'"
  exit 1
fi

echo "Latest successful run ID for workflow '$workflowFile' is $workflowRunId."

# List artifacts for that run.
artifactsJson=$(curl -s -H "Accept: application/vnd.github+json" -H "Authorization: Bearer ${GITHUB_TOKEN}" \
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
curl --fail-with-body -L -o "$outFilename" "$downloadUrl" -H "Authorization: Bearer ${GITHUB_TOKEN}" || {
  # When failed, print response body.
  cat "$outFilename"
  echo "::error::Failed to download artifact '$artifactName' from workflow '$workflowFile'."
  exit 1
}

echo "Downloaded artifact '${artifactName}' to $outFilename."

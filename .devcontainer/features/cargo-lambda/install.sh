#!/bin/bash

set -e -u -o pipefail

USERNAME="${USERNAME:-"vscode"}"

curl -fsSL https://cargo-lambda.info/install.sh | sh

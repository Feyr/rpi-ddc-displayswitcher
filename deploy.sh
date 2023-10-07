#!/bin/bash

set -o errexit
set -o nounset
set -o pipefail
set -o xtrace

readonly TARGET_HOST=pi@raspberrypi
readonly TARGET_PATH=/home/pi/hello-world
readonly SOURCE_PATH=./target/release/hello-world

cargo build --release
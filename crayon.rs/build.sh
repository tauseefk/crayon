#!/bin/bash

set -euo pipefail

TARGET=bundler
OUTDIR=../../www/crayon

wasm-pack build crayon --target $TARGET --release --out-dir $OUTDIR

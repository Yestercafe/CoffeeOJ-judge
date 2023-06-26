#!/usr/bin/env bash

tmux new-session -d "cargo run | ./node_modules/bunyan/bin/bunyan" && \
  tmux split-window -h "cd web && npm install && npm run dev" && \
  tmux -2 attach-session -d


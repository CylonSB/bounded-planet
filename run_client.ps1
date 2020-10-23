#!/bin/sh
cargo run --bin client -- --cert "./certs/cert.pem" --url "quic://localhost:4433" --accept_any
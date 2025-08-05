.PHONY: build
build:
	cargo build

.PHONY: format
format:
	cargo fmt

.PHONY: http.dev
http.dev:
	RUST_LOG=debug cargo watch -s "cargo run serve ./data --host 0.0.0.0 --url-prefix http://localhost:8080 --auth y2LQ3pUncnycVjJYUXM1sHF06GEP9dlUFdEHri4T"

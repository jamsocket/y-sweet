.PHONY: build
build:
	cargo build

.PHONY: format
format:
	cargo fmt

.PHONY: http.dev
http.dev:
	RUST_LOG=debug cargo watch -s "cargo run serve ./data --host 0.0.0.0 --url-prefix http://localhost:8080 --auth y2LQ3pUncnycVjJYUXM1sHF06GEP9dlUFdEHri4T"

.PHONY: http.dev.s3
http.dev.s3:
	RUST_LOG=debug cargo watch -s "cargo run serve s3://b-dev-kokuyo-digital-campus-be-ws --host 0.0.0.0 --url-prefix https://page-000.dev-campusforschool.com --auth y2LQ3pUncnycVjJYUXM1sHF06GEP9dlUFdEHri4T"
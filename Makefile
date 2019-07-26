dev: src/main.rs Cargo.toml Cargo.lock
	rusty-tags -o vi -s src
	cargo build
V := $(shell ruby -e 'puts STDIN.read[/version *= *"(.*?)"/, 1]' < Cargo.toml)
release:
	cargo build --release --target x86_64-unknown-linux-musl
	mkdir -p dist/0xbtc-miner-feed_v$V
	cp target/x86_64-unknown-linux-musl/release/0xbtc-miner-feed dist/0xbtc-miner-feed_v$V/
	cp config.example.toml dist/0xbtc-miner-feed_v$V/
	(cd dist; zip -r - 0xbtc-miner-feed_v$V >0xbtc-miner-feed_v$V.zip)
	@echo "wrote dist/0xbtc-miner-feed_v$V.zip"

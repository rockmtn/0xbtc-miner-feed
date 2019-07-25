# 0xbtc-miner-feed

Low-latency mining parameters server for 0xBitcoin miners (Rust).

## Build

Install Rust: <https://www.rust-lang.org/tools/install>

Build for development:
```
cargo build
# or if you don't want debug features:
cargo build --release
```

## Configure

Example configuration is here: [config.example.toml](config.example.toml)

```
cp config.example.toml config.toml
# then edit config.toml
```

## Running

```
cargo run
# or (if built in dev mode [default])
./target/debug/0xbtc-miner-feed
# or (built in release mode)
./target/release/0xbtc-miner-feed
```

### Running as a system service

Modify the file [systemd/0xbtc-miner-feed.service](systemd/0xbtc-miner-feed.service) so it has the right username and paths for your system.

```
sudo cp systemd/0xbtc-miner-feed.service /etc/systemd/system/
sudo systemctl enable 0xbtc-miner-feed
sudo systemctl start 0xbtc-miner-feed
```

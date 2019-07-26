# 0xbtc-miner-feed

Low-latency mining parameters server for 0xBitcoin miners (Rust).

## Download

- Linux static binary: [0xbtc-miner-feed_v0.1.0.zip](https://github.com/rockmtn/0xbtc-miner-feed/releases/download/v0.1.0/0xbtc-miner-feed_v0.1.0.zip)

## Configure

After downloading, copy `config.example.toml` to `config.toml` and edit the values in the latter file.

```
cp config.example.toml config.toml
# then edit config.toml
```

- Example config: [config.example.toml](config.example.toml)

## Building from source

Install Rust: <https://www.rust-lang.org/tools/install>

Build for development:
```
cargo build
# or if you don't want debug features:
cargo build --release
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

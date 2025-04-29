![rustc](https://img.shields.io/badge/rustc-1.85.0-blue.svg)
[![codecov](https://codecov.io/gh/averageeucplayer/lost-metrics-processor/graph/badge.svg?token=HHRGYYUNM2)](https://codecov.io/gh/averageeucplayer/lost-metrics-processor)
![CI](https://github.com/averageeucplayer/lost-metrics-processor/actions/workflows/ci.yml/badge.svg)

# 📊 Lost Metrics Processor  

A library that provides abstraction and utility methods for processing packets.

## 📦 Installation & Setup

### 1️⃣ **Clone the Repository**

```sh
git clone https://github.com/averageeucplayer/lost-metrics-processor.git
```

### 2️⃣ Add to Cargo.toml

```toml
[dependencies]
lost-metrics-processor = { git = "https://github.com/averageeucplayer/lost-metrics-processor" }
```

### 3️⃣ Consume in your project

```rust
```

## 🧪 Coverage

```sh
cargo llvm-cov cargo llvm-cov --ignore-filename-regex '.*(event_emitter|stats_api|settings_manager|local_player_store|register_listeners|test_utils|background_worker|event_listener|file_system|packet_sniffer|region_store|flags|start|register_listeners|heartbeat_api|lost-metrics-core|lost-metrics-data|lost-metrics-misc|lost-metrics-store|lost-metrics-sniffer-stub).*'  --summary-only 
```

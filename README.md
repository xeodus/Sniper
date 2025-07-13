# Sniper Bot

![Rust](https://img.shields.io/badge/Rust-006845?style=flat&logo=rust&logoColor=white&labelColor=333333)
![Build Status](https://github.com/xeodus/Sniper/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/License-MIT%20-white.svg)

This is an implementation of a trade bot designed for low-latency environments like cryto exchanges. It leverages on robust market algorithms and statistical models to take high-frequency trades. The bot takes care of various market factors both in highly volatile and side ways moving markets. The advanced market-making strategies and risk-management protocols are designed to secure sustained growth and minimize market mishaps. The bot is written from scratch in ```Rust```. The bot is being primarily developed for ```KuCoin``` exchange but hope to deliver for other exchanges too.

Sniper is still in its early stages of development, and the code is subject to change. The bot is designed to be memory-safe, concurrent, and asynchronous, making it suitable for high-performance trading applications.

## Features
- [x] **Advanced Market-Making Algorithms**
- [x] **Risk Management Protocols**
- [x] **KuCoin API Integration**
- [x] **WebSocket Integration**
- [x] **Concurrency Module**
- [x] **Memory-Safety**
- [x] **Asynchronous Operations**
- [x] **Unit Tests**

## Pending Work

- [] More efficient error handling
- [] Seemless and blazing-fast WebSocket Integration
- [] Model deployment

## Setup Guide

- **Requirements:** 
- [x] ```Rust 1.70+```
- [x] ```KuCoin``` ```API key```, ```Secret key```, and ```Passphrase```

- Create a ```.env``` file and host all your safety credentials there:

```bash
# .env
API_KEY="Your_API_key"
SECRET_KEY="Your_secret_key"
PASSPHRASE="Your_passphrase"
```

- Ensure you have Rust installed. If not, install it from [rustup.rs](https://rustup.rs)

Project setup:

```bash
    git clone https://github.com/xeodus/Sniper.git
    cd Sniper
```
To run unit tests:

```bash
    cd src/tests
    cargo test
```

Test Build:

```bash
    cargo build --release
    cargo run
```

Cheers 🍻

Project is still under-development, everything is still in its trial phase..
Hope to deploy soon 🤞

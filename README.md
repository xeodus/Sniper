# Trade Bot 

## Introduction

![Rust](https://img.shields.io/badge/Rust-006845?style=flat&logo=rust&logoColor=white&labelColor=333333)
![Build Status](https://github.com/xeodus/Sniper/actions/workflows/CI/badge.svg)

This is an implementation of a trade bot designed for low-latency environments like cryto exchanges. It leverages on robust market algorithms and statistical models to take high-frequency trades. The bot takes care of various market factors both in highly volatile and side ways moving markets. The advanced market-making strategies and risk-management protocols are designed to secure sustained growth and minimize market mishaps. The bot is written from scratch in ```Rust```. The bot is being primarily developed for ```KuCoin``` exchange but hope to deliver for other exchanges too.

## Strategy

- [x] Read candle stick patterns & historical data
- [x] ```EMA``` & ```SMA``` calculations
- [x] ```Bollinger Band``` calculations
- [x] Market-based personal strategies
- [x] Market-trend algorithms

## Risk-Management Protocol

- [x] Maxium drawdown percentage and potential loss protocols
- [x] Advanced stop loss protocol
- [x] Position sizing constraints
- [x] Portfolio Risk-Manager

## KuCoin API Integration

- [x] API & secret key integration
- [x] Secure ```HMAC-SHA256``` authentication
- [x] Real time market data and order execution using WebSockets

## Rust-Powered Performance

- [x] ```Lock-free``` nature & ```Concurrency``` module
- [x] Rust's speed and safety for high-performance financial applications
- [x] ```Memory-Safety```

## Pending Work

- [] More efficient error handling
- [] Advanced Market-Making Algorithm integration
- [] Seemless and blazing-fast WebSocket Integration
- [] Backtesting
- [] Model deployment

## Setup Guide

- **Requirements:** 
- [x] ```Rust 1.65+```
- [x] ```KuCoin``` API, secret keys, and passphrase

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
To run tests:

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

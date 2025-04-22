# Trade Bot 

![Rust](https://img.shields.io/badge/Rust-006845?style=flat&logo=rust&logoColor=white&labelColor=333333)
![License: MIT](https://img.shields.io/badge/License-MIT-red.svg)

## Table of Contents
- [Introduction](#Introduction)
- [Features](#Core_Features)
- [Setup Guide](#Setup_Guide)

## Introduction

This is a trade bot designed for low latency environments like cryto exchanges. It leverages on robust market algorithms and statitical models to take high-frequency trades, capitalizing on market trends and volatility. The bot is written from scratch in Rust.

**Project Underdevelopement..**

## Core Features

- **Order Book Imbalance Strategy:** 

```bash
let imbalance = (bid_pressure - bid_asks) / (bid_pressure + bid_asks);
```

- **Risk Manangement Protocol:** 

- [x] 1% stop loss protocol
- [x] Position sizing constraints

- **Binance API Integration:** 

- [x] Secure HMAC-SHA256 authentication
- [x] Real time market data and order execution

- **Rust-Powered Performance:**

- [x] Leverages Rust's speed and safety for high-performance financial applications

- **Pending Work:**

- [] More efficient error handling
- [] Backtesting
- [] Model deployment

## Setup Guide

- **Requirements:** 
- [x] Rust 1.65+
- [x] Binance API and secret keys

```bash
# .env
API_KEY="Your_API_key"
SECRET_KEY="Your_secret_key"
```

- Ensure you have Rust installed. If not, install it from [rustup.rs](https://rustup.rs)

Clone the repository:
```bash
git clone https://github.com/xeodus/moon-sniper.git
cd moon-sniper

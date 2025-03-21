# Trading Risk Manager

![Rust](https://img.shields.io/badge/Rust-006845?style=flat&logo=rust&logoColor=white&labelColor=333333)
![Crate Version](https://img.shields.io/badge/crate-0.1.0-green.svg)
![Welcome Badge](https://img.shields.io/badge/Welcome-Devs-yellow.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-red.svg)

## Table of Contents
- [Introduction](#Introduction)
- [Features](#Core_Features)
- [Setup Guide](#Setup_Guide)

## Introduction

A robust risk management module for trading applications written in Rust. Our mission is to leverage blockchain's transparency to create an automated, auditable and impact focused trading ecosystem.


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

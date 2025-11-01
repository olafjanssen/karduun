# Notary

Cryptographic signing and timestamping.

## Overview

Notary provides cryptographic signatures and timestamping services for cards, enabling authenticity verification and tamper detection.

## Status

⚠️ **Phase 4 Tool** - Currently stubbed, full implementation coming soon.

## Planned Commands

- `notary sign` - Ed25519 sign cards
- `notary verify` - Verify signatures
- `notary timestamp` - OpenTimestamps integration

## Installation

```bash
cargo install --path notary
# or
cargo build --release --bin notary
```


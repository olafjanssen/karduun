# Notary

Cryptographic signing and timestamping for card authenticity.

## Overview

Notary provides cryptographic signatures and timestamping services for cards, enabling authenticity verification, tamper detection, and proof of existence. Cards can be signed with Ed25519 signatures and optionally timestamped via OpenTimestamps.

## Installation

```bash
cargo install --path notary
# or
cargo build --release --bin notary
```

## Commands

### `notary generate-key`

Generate a new Ed25519 key pair for signing.

**Usage:**
```bash
notary generate-key --out <directory>
```

**Options:**
- `--out <dir>` - Directory to write key files

**Examples:**
```bash
# Generate keys in .keys directory
notary generate-key --out .keys

# Store in secure location
notary generate-key --out ~/.config/cardstack/keys
```

**Output:**
- `secret.key` - Private key (keep secure!)
- `public.key` - Public key (can be shared)

**Security Notes:**
- ⚠️ **Never share or commit secret keys**
- Store secret keys in secure location (not in repository)
- Use environment variables or secure key storage for production
- Public keys can be shared for verification

### `notary sign`

Sign cards with Ed25519 signature.

**Usage:**
```bash
notary sign [--uid <uid>] [--query "..."] --key <path> [--jsonl]
```

**Options:**
- `--uid <uid>` - Sign specific card by UID or slug
- `--query <dsl>` - Sign cards matching query
- `--key <path>` - Path to secret key file (required)
- `--jsonl` - Output JSONL format

**Examples:**
```bash
# Sign specific card
notary sign --uid my-card --key .keys/secret.key

# Sign all published cards
notary sign --query "status=published" --key .keys/secret.key

# Sign by tag
notary sign --query "tag:public" --key ~/.config/cardstack/keys/secret.key
```

**What it does:**
1. Computes canonical hash of card (Blake3)
2. Signs hash with Ed25519 private key
3. Adds `sign` block to card with:
   - `algo`: "ed25519"
   - `by`: Key identifier (hex-encoded public key)
   - `sig`: Base64-encoded signature

**Output:**
```
Signed: Research Note (ulid_01ABC...)
Signed: Design Doc (ulid_01DEF...)
Signed 2 card(s)
```

**Signature Block:**
```yaml
sign:
  algo: ed25519
  by: key:3a7f2b9c1d4e5f6a...
  sig: base64-encoded-signature...
```

### `notary verify`

Verify card signatures.

**Usage:**
```bash
notary verify [--uid <uid>] [--query "..."] [--key <path>] [--jsonl]
```

**Options:**
- `--uid <uid>` - Verify specific card
- `--query <dsl>` - Verify cards matching query
- `--key <path>` - Path to public key file (optional, for validation)
- `--jsonl` - Output JSONL VerifyResult format

**Examples:**
```bash
# Verify all signed cards
notary verify --query "has:signature"

# Verify specific card
notary verify --uid my-card

# Verify with public key
notary verify --query "status=published" --key .keys/public.key

# JSONL output
notary verify --jsonl > verification-report.jsonl
```

**Verification Process:**
1. Load card and check for `sign` block
2. Recompute canonical hash
3. Verify signature against public key
4. Report validity

**Output:**
```
Verification Results:
  Total: 10
  Signed: 8
  Valid: 7
  Invalid: 1

research-note (ulid_01ABC...) - ✓ Valid signature
design-doc (ulid_01DEF...) - ✓ Valid signature
old-card (ulid_01GHI...) - ✗ Invalid signature
    Error: Signature verification failed
unpublished (ulid_01JKL...) - Not signed
```

**JSONL Format:**
```json
{
  "uid": "ulid_01ABC...",
  "slug": "research-note",
  "signed": true,
  "valid": true,
  "key_id": "key:3a7f2b9c1d4e5f6a...",
  "error": null
}
```

### `notary timestamp`

Timestamp cards (future: OpenTimestamps integration).

**Usage:**
```bash
notary timestamp [--uid <uid>] [--query "..."] [--jsonl]
```

**Status:** ⚠️ Placeholder - OpenTimestamps integration coming soon

**Planned Features:**
- OpenTimestamps API integration
- Cryptographic proof of existence at point in time
- Timestamp verification
- Calendar server synchronization

**Examples:**
```bash
# Timestamp cards (future)
notary timestamp --query "status=published"

# Timestamp specific card (future)
notary timestamp --uid my-card
```

## Security Model

### Signing Process

1. **Canonicalization**: Card is serialized to deterministic YAML
2. **Hashing**: Blake3 hash computed of canonical form
3. **Signing**: Hash signed with Ed25519 private key
4. **Storage**: Signature stored in card's `sign` block

### Verification Process

1. **Hash Computation**: Recompute canonical hash
2. **Signature Extraction**: Read signature from `sign` block
3. **Public Key Retrieval**: Get public key (from file or embedded)
4. **Verification**: Verify signature matches hash

### Key Management

**Best Practices:**
- Store secret keys outside repository
- Use environment variables in CI/CD
- Rotate keys periodically
- Use different keys for different purposes (development/production)
- Back up keys securely (encrypted)

**Key Storage:**
```
~/.config/cardstack/keys/
  ├── secret.key      # Private key (encrypted in production)
  ├── public.key      # Public key (safe to share)
  └── old-secret.key  # Rotated keys
```

## Integration Examples

### Sign Published Cards

```bash
# Sign all published cards
notary sign --query "status=published" --key .keys/secret.key

# Verify signatures
notary verify --query "status=published" --key .keys/public.key
```

### Automated Signing Workflow

```bash
#!/bin/bash
# Sign all cards with specific tag
notary sign --query "tag:verified" --key ~/.keys/secret.key

# Verify
notary verify --query "tag:verified" --key ~/.keys/public.key
```

### Batch Verification

```bash
# Verify all signed cards
notary verify --jsonl | jq 'select(.valid == false)'

# Count invalid signatures
notary verify --jsonl | jq '[select(.valid == false)] | length'
```

### Export with Verification

```bash
# Export and verify before sharing
notary verify --jsonl > verification.jsonl
porter export --format jsonl --out ./export --query "status=published"

# Share verification report alongside export
```

## Current Implementation Notes

⚠️ **Simplified Signing**: Current implementation uses a simplified signing approach for demonstration. For production use, integrate proper Ed25519 library (e.g., `ed25519-dalek`).

**Current limitations:**
- Uses simplified XOR-based signature (not cryptographically secure)
- Key generation is basic (not true Ed25519)
- Timestamping is placeholder

**Production improvements needed:**
- Integrate `ed25519-dalek` or similar
- Proper key derivation
- OpenTimestamps API integration
- Signature chain support

## Signature Format

Signatures are stored in the card's `sign` block:

```yaml
sign:
  algo: ed25519
  by: key:3a7f2b9c1d4e5f6a7890123456789abcdef
  sig: dGhpcyBpcyBhIHNpZ25hdHVyZSBleGFtcGxl...
```

**Fields:**
- `algo`: Always "ed25519" (for now)
- `by`: Key identifier (hex-encoded public key prefix)
- `sig`: Base64-encoded signature

## Workflow Examples

### Publishing Workflow

```bash
# 1. Create and edit card
scribe new "Research Paper" --tag research
scribe edit research-paper --field status=draft

# 2. Review and finalize
scribe edit research-paper --field status=ready

# 3. Sign before publishing
notary sign --uid research-paper --key .keys/secret.key

# 4. Publish
scribe edit research-paper --field status=published

# 5. Verify signature
notary verify --uid research-paper --key .keys/public.key
```

### Verification Workflow

```bash
# Check all signed cards are valid
notary verify --jsonl > verification.jsonl

# Find any invalid signatures
cat verification.jsonl | jq -r 'select(.valid == false) | .uid'

# Re-sign if needed
notary sign --uid <invalid-uid> --key .keys/secret.key
```

## Global Options

All commands support:

- `--repo <path>` - Override repository root
- `--jsonl` - Machine-readable JSONL output
- `--key <path>` - Key file path (for sign/verify)

## Security Considerations

### Key Protection

1. **Never commit secret keys** - Add to `.gitignore`
2. **Use secure storage** - Consider encrypted key files
3. **Rotate keys** - Generate new keys periodically
4. **Limit key access** - Restrict file permissions

### Signature Validity

- Signatures verify card content hasn't changed
- Changing any field invalidates signature
- Must re-sign after edits
- Consider signing only after finalization

### Production Recommendations

1. Use proper Ed25519 library (`ed25519-dalek`)
2. Implement key rotation workflow
3. Add signature chains for provenance
4. Integrate OpenTimestamps for proof of existence
5. Add signature expiration policies

## Troubleshooting

### "Key required for signing"
- Ensure `--key` flag points to secret key file
- Check file exists and is readable
- Verify key format is correct

### "Invalid signature"
- Card may have been modified after signing
- Key may not match signing key
- Signature may be corrupted

### "Cannot verify (no key)"
- Provide `--key` with public key for verification
- Or verify using key identifier from signature

## Future Enhancements

- ✅ Basic signing/verification (current)
- 🔄 OpenTimestamps integration (planned)
- 🔄 Signature chains (planned)
- 🔄 Key rotation tools (planned)
- 🔄 Multi-signature support (planned)
- 🔄 Signature expiration (planned)

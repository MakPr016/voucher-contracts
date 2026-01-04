# Git Voucher - Solana Contracts

The on-chain Anchor program that powers the Git Voucher escrow system. It manages organizational wallets, locks funds for vouchers, and handles secure claiming.

## ⚡ Key Instructions

- `initialize_organization`: Registers a GitHub Organization/User on-chain.
- `deposit`: Adds SOL to the organization's escrow balance.
- `create_voucher`: Locks funds and creates a claimable voucher for a specific GitHub user ID.
- `claim_voucher`: Releases funds to the recipient (requires proof of identity via the frontend).
- `add_maintainer`: Authorizes other wallets to create vouchers on behalf of the organization.

## 🛠 Tech Stack

- **Language:** Rust
- **Framework:** Anchor 0.30.1
- **Network:** Solana Devnet (currently)

## 📦 Setup & Testing

1. **Prerequisites:**
   - Install Rust and Cargo.
   - Install Solana CLI.
   - Install Anchor CLI (`avm`).

2. **Install dependencies:**
   ```bash
   yarn install
   ```

3. **Build the program:**
   ```bash
   anchor build
   ```

4. **Run tests:**
   This runs the integration tests located in `tests/git-voucher-escrow.ts`.
   ```bash
   anchor test
   ```

## 📜 Deployment Info (Devnet)

- **Program ID:** `8iRpzhFJF4PJnhyKZRDXk6B3TKjxQGEX6kcsteYq77iR`
- **IDL:** Located in `target/idl/git_voucher_escrow.json` after building.

## 🔐 Account Structure

- **OrganizationEscrow:** Holds the total balance and list of maintainers.
- **VoucherEscrow:** A PDA (Program Derived Address) that holds the state and funds for a single voucher until claimed, expired, or cancelled.

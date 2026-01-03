import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GitVoucherEscrow } from "../target/types/git_voucher_escrow";
import { expect } from "chai";

describe("git-voucher-escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.GitVoucherEscrow as Program<GitVoucherEscrow>;
  
  const orgGithubId = new anchor.BN(12345);
  
  const [orgPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("organization"), orgGithubId.toArrayLike(Buffer, "le", 8)],
    program.programId
  );

  it("Initializes organization", async () => {
    try {
      const tx = await program.methods
        .initializeOrganization(orgGithubId)
        .accounts({
          organization: orgPDA,
          admin: provider.wallet.publicKey,
        })
        .rpc();

      console.log("✅ Organization initialized:", tx);
    } catch (error) {
      console.log("⚠️ Organization already exists, fetching...");
    }

    const org = await program.account.organizationEscrow.fetch(orgPDA);
    expect(org.orgGithubId.toNumber()).to.equal(12345);
    console.log("   Balance:", org.balance.toNumber() / 1_000_000_000, "SOL");
  });

  it("Deposits funds", async () => {
    const depositAmount = new anchor.BN(1_000_000_000);

    await program.methods
      .deposit(depositAmount)
      .accounts({
        organization: orgPDA,
        depositor: provider.wallet.publicKey,
      })
      .rpc();

    const org = await program.account.organizationEscrow.fetch(orgPDA);
    console.log("✅ Deposited 1 SOL. New balance:", org.balance.toNumber() / 1_000_000_000, "SOL");
  });

  it("Adds maintainer", async () => {
    const maintainerKeypair = anchor.web3.Keypair.generate();

    await program.methods
      .addMaintainer(maintainerKeypair.publicKey)
      .accounts({
        organization: orgPDA,
        admin: provider.wallet.publicKey,
      })
      .rpc();

    const org = await program.account.organizationEscrow.fetch(orgPDA);
    console.log("✅ Maintainer added. Total maintainers:", org.maintainers.length);
  });

  it("Creates voucher", async () => {
    try {
      await program.methods
        .addMaintainer(provider.wallet.publicKey)
        .accounts({
          organization: orgPDA,
          admin: provider.wallet.publicKey,
        })
        .rpc();
    } catch (error) {
      console.log("⚠️ Maintainer already added");
    }

    const voucherId = `voucher-${Date.now()}`;
    const recipientGithubId = new anchor.BN(67890);
    const voucherAmount = new anchor.BN(100_000_000);
    const metadata = JSON.stringify({ repo: "owner/repo", pr: 123 });

    const [voucherPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("voucher"), Buffer.from(voucherId)],
      program.programId
    );

    await program.methods
      .createVoucher(voucherId, recipientGithubId, voucherAmount, metadata)
      .accounts({
        organization: orgPDA,
        voucher: voucherPDA,
        maintainer: provider.wallet.publicKey,
      })
      .rpc();

    const voucher = await program.account.voucherEscrow.fetch(voucherPDA);
    console.log("✅ Voucher created:", voucherId);
    console.log("   Amount:", voucher.amount.toNumber() / 1_000_000_000, "SOL");
    console.log("   Recipient GitHub ID:", voucher.recipientGithubId.toNumber());
    
    expect(voucher.voucherId).to.equal(voucherId);
    expect(voucher.recipientGithubId.toNumber()).to.equal(67890);
    expect(voucher.amount.toNumber()).to.equal(100_000_000);
  });
});

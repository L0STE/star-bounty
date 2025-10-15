# Star Bounty Protocol - Technical Documentation

This protocol extends an existing token sale contract to enable automated fee distribution from a Meteora dAMM liquidity position to investors based on their remaining vested tokens.

## State Architecture

### State Accounts
```rust
// 1. Creator - Retrofitted from existing contract
#[account]
pub struct Creator {
    pub bump: [u8; 1],
    pub streams: u8,     // Critical: Enables permissionless claiming
}

// 2. InvestorFeePositionOwner - Owns the Meteora position
#[account]
pub struct InvestorFeePositionOwner {
    pub associated_mint: Pubkey,
    pub last_claimed_at: i64,     // 24h cooldown
    pub bump: [u8; 1],
}
```

---

## Creator State

We had to retrofit the current Creator/TokenSale account to tracks the number of vesting streams created for the token sale to support the new fee distribution mechanism.

**Problem**: How do you make fee claiming permissionless making sure that all investors addresses are passed without clogging PDAs with Pubkey Lists (that fills up to quickly)?

**Solution**: Use an **index-based PDA derivation system**.
```rust
// Stream counter enables deterministic PDA derivation
pub streams: u8  // Incremented each time a stream is created
```

#### How It Works
```
When creating streams:
├─ Stream #0 → Metadata PDA = ["metadata", creator, [0]]
├─ Stream #1 → Metadata PDA = ["metadata", creator, [1]]
├─ Stream #2 → Metadata PDA = ["metadata", creator, [2]]
└─ ...

When claiming fees (permissionless):
├─ Read creator.streams = 3
├─ Derive ["metadata", creator, [0]] ✓
├─ Derive ["metadata", creator, [1]] ✓
├─ Derive ["metadata", creator, [2]] ✓
└─ Validate each PDA exists and is correct
```

Anyone can trigger fee distribution making sure that all investors address are present.

---

## InvestorFeePositionOwner State

This state owns the Meteora dAMM position that collects trading fees and track the `last_claimed` field to make sure that claims are after 24h.

**Current Design**: Separate `InvestorFeePositionOwner` account

**Alternative**: Merge into `Creator` state
```rust
pub struct Creator {
    pub bump: [u8; 1],
    pub streams: u8,
    pub last_claimed_at: i64,  // ← Add this field
}
```

My raccomandation is to merge it into the `Creator` state and just derive the `position_nft` with different seeds.

---

## Instructions

To showcase how to create the pool and the streamflow accounts based on the new refactored states I added those 2 instructions that can be used as example.

Note: In the tests the `mint_b` rapresent USDC but it should be flipped to your token. Currently this is not possible because of the contraint present [here](https://github.com/MeteoraAg/damm-v2/blob/689a3264484799d833c505523f4ff4e4990690aa/programs/cp-amm/src/constants.rs#L172) that will be deleted with version V0.1.6 (confirmed by the team) since it's a bug that I discovered while building this contract. 

---

## 🔄 Claim Fees Instruction

### The Permissionless Design
```rust
pub fn claim_fees(
    ctx: Context>
) -> Result {
    ctx.accounts.claim()?;
    ctx.accounts.distribute(ctx.remaining_accounts)  // ← Streams passed here
}
```

### How Permissionless Claiming Works
```rust
// 1. Caller reads creator.streams (on-chain)
let stream_count = creator.streams;  // e.g., 5

// 2. Caller derives all stream PDAs
for i in 0..stream_count {
    let (metadata_pda, _) = Pubkey::find_program_address(
        &[b"metadata", creator.key().as_ref(), &[i]],
        &program_id
    );
    remaining_accounts.push(metadata_pda);
    remaining_accounts.push(recipient_token_account);
}

// 3. Call claim_fees with derived accounts
claim_fees(ctx, remaining_accounts);
```

### Validation Inside Claim
```rust
pub fn distribute(&mut self, remaining_accounts: &[AccountInfo]) -> Result {
    let streams = self.creator.streams as usize;
    
    // Verify correct number of accounts passed
    require_eq!(remaining_accounts.len(), streams * 2);  // metadata + token account

    // For each stream, validate the PDA
    for i in 0..streams {
        let metadata_account = &remaining_accounts[i * 2];
        
        // Derive expected PDA
        let (expected_stream, _) = Pubkey::find_program_address(&[
            b"metadata",
            self.creator.to_account_info().key.as_ref(),
            &[i as u8],  // ← Uses index from creator.streams
        ], &crate::ID);
        
        // Verify it matches
        require_eq!(expected_stream, metadata_account.key());
        
        // Deserialize and use Streamflow data
        let stream_metadata = streamflow_sdk::state::Contract::deserialize(
            &mut metadata_account.try_borrow_data()?.as_ref()
        )?;
        
        // Calculate locked amount and distribute...
    }
}
```

### 📐 Fee Distribution Formula

```rust
// 1. Calculate total locked across all investors
total_locked = Σ(each_investor_locked_amount)

// 2. Calculate initial locked (at vesting start)
initial_locked = Σ(each_investor_initial_amount)

// 3. Calculate locked ratio (f_locked)
f_locked = total_locked / initial_locked

// 4. Determine investor share (capped at daily limit)
investor_share = min(daily_limit, f_locked × 100%)

// 5. Calculate distributable fees
distributable = min(fees_claimed × investor_share, DAILY_CAP)

// 6. Each investor receives proportional share
investor_i_share = distributable × (investor_i_locked / total_locked)

// 7. Creator receives remainder
creator_share = fees_claimed - total_distributed
```

Note: The caps are now hardcoded to be the same for each `claim_fee` instruction, but it's completely possible to save them into the state and use those as limits to make it more programmable for each launch.

Note 2: We use all the data from the streamflow account to make it safe (that's why we check the `total_inital_locked` calculating the `net_deposited` at creation, this is safe only if topup are turned off)

---

## Tests

We use surfpool for testing, replicating a mainnet environment so it's just a drag and drop exercise for integration: 

To test, [download](https://docs.surfpool.run/) surfpool and then run:

```surfpool start```

And then to test run

```anchor test```

Note: since we have runbooks in it, the program will be automatically deployed, you'll probably need to just change the program ID in the `lib.rs`

Note 2: each integration has it's separate ts helper file to make tests cleaner

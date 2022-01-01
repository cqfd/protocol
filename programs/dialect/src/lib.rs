use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use solana_program::program_option::COption;

declare_id!("2YFyZAg8rBtuvzFFiGvXwPHFAQJ2FXZoS7bYCKticpjk");

/*
Entrypoints
*/
#[program]
pub mod dialect {
    use super::*;

    /*
    User metadata
    */

    pub fn create_metadata(ctx: Context<CreateMetadata>, _metadata_nonce: u8) -> ProgramResult {
        let metadata = &mut ctx.accounts.metadata.load_init()?;
        metadata.user = ctx.accounts.user.key();
        metadata.device_token = DeviceToken::default();
        metadata.subscriptions = [Subscription::default(); 32];
        Ok(())
    }

    /*
    Dialects
    */

    pub fn create_dialect(
        ctx: Context<CreateDialect>,
        _dialect_nonce: u8,
        scopes: [[bool; 2]; 2],
    ) -> ProgramResult {
        // TODO: Assert owner in members
        let dialect_loader = &ctx.accounts.dialect;
        let mut dialect = dialect_loader.load_init()?;
        let owner = &mut ctx.accounts.owner;
        let members = [&mut ctx.accounts.member0, &mut ctx.accounts.member1];

        dialect.members = [
            Member {
                public_key: *members[0].key,
                scopes: scopes[0],
            },
            Member {
                public_key: *members[1].key,
                scopes: scopes[1],
            },
        ];

        dialect.messages = [Message::default(); 32];
        dialect.next_message_idx = 0;
        dialect.last_message_timestamp = Clock::get()?.unix_timestamp as u32; // TODO: Do this properly or use i64

        emit!(CreateDialectEvent {
            dialect: dialect_loader.key(),
        });

        Ok(())
    }

    pub fn update_device_token(
        ctx: Context<UpdateDeviceToken>,
        _dialect_nonce: u8,
        encrypted_device_token_array: [u8; 64], // TODO: Reduce this to 48 in payload
        encryption_nonce: [u8; 24],
    ) -> ProgramResult {
        // TODO: Confirm unsetting works
        let metadata = &mut ctx.accounts.metadata.load_mut()?;
        let arr = slice(&encrypted_device_token_array);
        metadata.device_token = DeviceToken {
            encrypted_array: arr,
            nonce: encryption_nonce,
        };
        Ok(())
    }

    pub fn subscribe_user(
        ctx: Context<SubscribeUser>,
        _dialect_nonce: u8,
        _metadata_nonce: u8,
    ) -> ProgramResult {
        let dialect = &mut ctx.accounts.dialect;
        let metadata_loader = &mut ctx.accounts.metadata;
        let metadata = &mut metadata_loader.load_mut()?;
        let num_subscriptions = metadata
            .subscriptions
            .iter()
            .filter(|s| is_present(s))
            .count();
        // TODO: handle max subscriptions
        if num_subscriptions < 32 {
            metadata.subscriptions[num_subscriptions] = Subscription {
                pubkey: dialect.key(),
                enabled: true,
            };
            emit!(SubscribeUserEvent {
                metadata: metadata_loader.key(),
                dialect: dialect.key()
            });
        } else {
            msg!("User already subscribed to 32 dialects");
        }
        Ok(())
    }

    pub fn send_message(
        ctx: Context<SendMessage>,
        _dialect_nonce: u8,
        text: [u8; 256],
    ) -> ProgramResult {
        let dialect_loader = &ctx.accounts.dialect;
        let mut dialect = dialect_loader.load_mut()?;
        let sender = &mut ctx.accounts.sender;
        let idx = dialect.next_message_idx;
        let timestamp = Clock::get()?.unix_timestamp as u32; // TODO: Do this properly or use i64
        dialect.messages[idx as usize] = Message {
            owner: *sender.key,
            text,
            timestamp,
        };
        dialect.next_message_idx = (dialect.next_message_idx + 1) % 32;
        dialect.last_message_timestamp = timestamp;
        emit!(SendMessageEvent {
            dialect: dialect_loader.key(),
            sender: *sender.key,
        });
        Ok(())
    }

    /*
    Mint Dialects
    */
    pub fn create_mint_dialect(
        ctx: Context<CreateMintDialect>,
        _dialect_nonce: u8,
    ) -> ProgramResult {
        let mint = &ctx.accounts.mint;
        let dialect = &mut ctx.accounts.dialect;
        dialect.mint = mint.key();
        Ok(())
    }

    /*
    Transfer test
    */

    pub fn transfer(ctx: Context<Transfer>, amount1: u64, amount2: u64) -> ProgramResult {
        let sender = &mut ctx.accounts.sender;
        let receiver1 = &mut ctx.accounts.receiver1;
        let receiver2 = &mut ctx.accounts.receiver2;
        let system_program = &ctx.accounts.system_program;
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                sender.key,
                receiver1.key,
                amount1,
            ),
            &[
                sender.to_account_info(),
                receiver1.to_account_info(),
                system_program.to_account_info(),
            ],
        )?;
        anchor_lang::solana_program::program::invoke(
            &anchor_lang::solana_program::system_instruction::transfer(
                sender.key,
                receiver2.key,
                amount2,
            ),
            &[
                sender.to_account_info(),
                receiver2.to_account_info(),
                system_program.to_account_info(),
            ],
        )?;
        Ok(())
    }
}

/*
Contexts
*/

#[derive(Accounts)]
#[instruction(metadata_nonce: u8)]
pub struct CreateMetadata<'info> {
    #[account(signer, mut)]
    pub user: AccountInfo<'info>,
    #[account(
        init,
        seeds = [
            b"metadata".as_ref(),
            user.key.as_ref(),
        ],
        bump = metadata_nonce,
        payer = user,
        // discriminator (8) + user + device_token + 32 x (subscription) = 1168
        space = 8 + 32 + 72 + (32 * 33),
    )]
    pub metadata: Loader<'info, MetadataAccount>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(metadata_nonce: u8)]
pub struct UpdateDeviceToken<'info> {
    #[account(signer, mut)]
    pub user: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            b"metadata".as_ref(),
            user.key.as_ref(),
        ],
        has_one = user, // TODO: Confirm if seeds solves this
        bump = metadata_nonce,
    )]
    pub metadata: Loader<'info, MetadataAccount>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(dialect_nonce: u8, metadata_nonce: u8)] // metadata0_nonce: u8, metadata1_nonce: u8)]
pub struct SubscribeUser<'info> {
    #[account(signer, mut)]
    pub signer: AccountInfo<'info>,
    // TOOD: Consider at some point enforcing user = signer
    pub user: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            b"metadata".as_ref(),
            user.key().as_ref(),
        ],
        bump = metadata_nonce,
        constraint = metadata
            .load()?
            .subscriptions
            .iter()
            .filter(|s| s.pubkey == dialect.key())
            .count() < 1 // no duplicate subscriptions
    )]
    pub metadata: Loader<'info, MetadataAccount>,
    pub dialect: AccountInfo<'info>, // we only need the pubkey, so AccountInfo is fine. TODO: is this a security risk?
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(dialect_nonce: u8)]
pub struct CreateDialect<'info> {
    #[account(signer, mut)] // mut is needed because they're the payer for PDA initialization
    // We dupe the owner in one of the members, since the members must be sorted
    pub owner: AccountInfo<'info>,
    pub member0: AccountInfo<'info>,
    // // TOOD: Set limit, or use remaining accounts for members
    pub member1: AccountInfo<'info>,
    // TODO: Support more users
    #[account(
        init,
        // TODO: Assert that owner is a member with admin privileges
        seeds = [
            b"dialect".as_ref(),
            member0.key().as_ref(),
            member1.key().as_ref(),
        ],
        constraint = member0.key().cmp(&member1.key()) == std::cmp::Ordering::Less, // n.b. asserts !eq as well
        bump = dialect_nonce,
        payer = owner,
        // NB: max space for PDA = 10240
        // space = discriminator + 2 * Member + 32 * Message
        space = 8 + 68 + 9344 + 1 + 4
    )]
    pub dialect: Loader<'info, DialectAccount>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Transfer<'info> {
    #[account(signer, mut)]
    pub sender: AccountInfo<'info>,
    #[account(mut)]
    pub receiver1: AccountInfo<'info>,
    #[account(mut)]
    pub receiver2: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(dialect_nonce: u8)]
pub struct SendMessage<'info> {
    #[account(
        signer,
        mut,
        constraint = dialect.load()?.members.iter().filter(|m| m.public_key == *sender.key && m.scopes[1] == true).count() > 0,
    )]
    pub sender: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [
            b"dialect".as_ref(),
            dialect.load()?.members[0].public_key.as_ref(),
            dialect.load()?.members[1].public_key.as_ref(),
        ],
        bump = dialect_nonce,
    )]
    pub dialect: Loader<'info, DialectAccount>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(dialect_nonce: u8)]
pub struct CreateMintDialect<'info> {
    #[account(signer, mut)] // mut is needed because they're the payer for PDA initialization
    pub mint_authority: AccountInfo<'info>, // The dialect owner must be the mint authority
    // TODO: Enforce that mint.mint_authority exists
    #[account(constraint = COption::Some(mint_authority.key()) == mint.mint_authority)]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        seeds = [b"dialect".as_ref(), mint.key().as_ref()],
        bump = dialect_nonce,
        payer = mint_authority,
        space = 512, // TODO: Choose space
    )]
    pub dialect: Account<'info, MintDialectAccount>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: AccountInfo<'info>,
}

/*
Accounts
*/

#[account(zero_copy)] // in anticipation of more subscriptions
#[derive(Default)]
pub struct MetadataAccount {
    // TODO: Add profile
    user: Pubkey,                      // 32
    device_token: DeviceToken,         // 32. TODO: Encrypt
    subscriptions: [Subscription; 32], // 4 * space(Subscription) TODO: More subscriptions
}

#[account(zero_copy)]
#[derive(Default)]
// TODO: Address 4kb stack frame limit with zero copy https://docs.solana.com/developing/on-chain-programs/overview#stack
pub struct DialectAccount {
    pub members: [Member; 2],        // 2 * Member = 68
    pub messages: [Message; 32],     // 32 * Message = 2176 (will be 9344 with message length 256)
    pub next_message_idx: u8,        // 1 -- index of next message (not the latest)
    pub last_message_timestamp: u32, // 4 -- timestamp of the last message sent, for sorting dialects
}

#[account]
#[derive(Default)]
pub struct MintDialectAccount {
    pub mint: Pubkey,
    // pub mint_authority: Pubkey, // TODO: Do we need this?
}

/*
Data
*/

#[zero_copy]
// space = 48 + 24 = 72
pub struct DeviceToken {
    pub encrypted_array: [u8; 48], // 32-byte token, 16-byte encryption overhead
    pub nonce: [u8; 24], // https://github.com/dchest/tweetnacl-js/blob/3389b7c9f00545e516a0fdafb324b7857cf1527d/nacl-fast.js#L2074
}

impl Default for DeviceToken {
    fn default() -> Self {
        DeviceToken {
            encrypted_array: [0; 48],
            nonce: [0; 24],
        }
    }
}

#[zero_copy]
#[derive(Default)]
// space = 33
pub struct Subscription {
    pub pubkey: Pubkey, // 32
    pub enabled: bool,  // 1
}

#[zero_copy]
#[derive(Default)]
// space = 34
pub struct Member {
    pub public_key: Pubkey, // 32
    // [Admin, Write]. [false, false] implies read-only
    pub scopes: [bool; 2], // 2
}

#[zero_copy]
// space = 292
pub struct Message {
    pub owner: Pubkey, // 32
    // max(u32) -> Sunday, February 7, 2106 6:28:15 AM
    // max(u64) -> Sunday, July 21, 2554 11:34:33 PM
    pub timestamp: u32,  // 4
    pub text: [u8; 256], // 256
}

impl Default for Message {
    fn default() -> Self {
        Self {
            owner: Pubkey::default(),
            text: [0; 256],
            timestamp: 0,
        }
    }
}

#[event]
pub struct CreateDialectEvent {
    pub dialect: Pubkey,
}

#[event]
pub struct SendMessageEvent {
    pub dialect: Pubkey,
    pub sender: Pubkey,
}

#[event]
pub struct SubscribeUserEvent {
    pub metadata: Pubkey,
    pub dialect: Pubkey,
}

/*
Helper functions
*/

pub fn is_present(subscription: &Subscription) -> bool {
    subscription.pubkey != Pubkey::default()
}

fn slice(input: &[u8]) -> [u8; 48] {
    // :sob:
    // TODO: Either use try_into a la https://stackoverflow.com/a/50080940, or retire the need to slice entirely by passing up [u8; 48] from client.
    [
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
        input[9], input[10], input[11], input[12], input[13], input[14], input[15], input[16],
        input[17], input[18], input[19], input[20], input[21], input[22], input[23], input[24],
        input[25], input[26], input[27], input[28], input[29], input[30], input[31], input[32],
        input[33], input[34], input[35], input[36], input[37], input[38], input[39], input[40],
        input[41], input[42], input[43], input[44], input[45], input[46], input[47],
    ]
}

mod err;
mod config;
mod utils;
pub mod generation;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Mint, MintTo, CloseAccount, Token, Burn};
use anchor_spl::{
    associated_token::{AssociatedToken}
};

use anchor_lang::solana_program::log::sol_log_compute_units;
use anchor_lang::solana_program::program::invoke_signed;
use mpl_token_metadata::instruction::create_metadata_accounts;
use crate::config::ConfigLine;
use crate::err::ErrorCode;
use anchor_lang::solana_program::sysvar;
use crate::generation::gen_vals::{roll_stat_per_level};
use crate::utils::instruction_check;
declare_id!("EqUAhUJLQdx2RqE3jbMXbQV6HCG865B8rePJfnCYD3vx");
/**
we need a contract which has a method which creates


> js
1. create a mint for the NFT with mint_authority as the owner
2. create a token account for the user for the mint
4. derive a PDA from the mint & the program which holds the metadata
 **/

const DECIMALS: u8 = 6;
const PREFIX: &[u8] = b"crypto-mons";
const CREATOR: &[u8] = b"creator";
const MON_STATE_SEED: &[u8] = b"mon-state";
const MON_TOKEN_SEED: &[u8] = b"mon-token";
const MASS_SEED: &[u8] = b"mass-token";
const ORDER_SEED: &[u8] = b"order-token";
const ENERGY_SEED: &[u8] = b"energy-token";
const STAT_TOKEN_STORE: &[u8] = b"stat-token-store";

#[program]
pub mod mon_maker {
    pub fn initialize(_ctx: Context<Initialize>, _game_creator_bump: u8, _mon_token_bump: u8, _mass_bump: u8, _energy_bump: u8, _order_bump: u8) -> ProgramResult {
        Ok(())
    }
    pub fn initialize_user(_ctx: Context<InitializeUser>, _game_creator_bump: u8, _mon_token_bump: u8, _mass_bump: u8, _energy_bump: u8, _order_bump: u8) -> ProgramResult {
        Ok(())
    }
    
    /** create tokens associated with stats of nft in-game character */
    pub fn mint_stat_tokens_to(ctx: Context<MintStatTokensTo>, amount: u64, _stat_seed: String, _mint_bump: u8, game_creator_bump: u8) -> ProgramResult {
        let seeds = &[PREFIX, &ctx.accounts.game_creator_auth.key().to_bytes()[..], CREATOR, &[game_creator_bump]];
        let signer = &[&seeds[..]];
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.stat_mint_pda.to_account_info(),
                to: ctx.accounts.user_stat_ata_pda.to_account_info(),
                authority: ctx.accounts.game_creator_pda.to_account_info(),
            },
            signer
        ), amount)?;
        Ok(())
    }
    
    /** initializing game data state holding account associated with nft */
    pub fn initialize_nft_mon_state(ctx: Context<InitializeNftMonState>,  _mon_state_bump: u8) -> ProgramResult {
        let mon_state = &mut ctx.accounts.mon_state;
        let now_ts = Clock::get().unwrap().unix_timestamp as u64;
        mon_state.seed = now_ts;
        Ok(())
    }
    
    /** creating new token (mint) for input user */
    pub fn initialize_nft_mint(ctx: Context<InitializeNftMint>, game_creator_bump: u8, _nft_bump: u8) -> ProgramResult {
        let seeds = &[PREFIX, &ctx.accounts.game_creator_auth.key().to_bytes()[..], CREATOR, &[game_creator_bump]];
        let signer = &[&seeds[..]];
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.nft_mint_pda.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.game_creator_pda.to_account_info(),
            },
            signer
        ), 1_u64)?;
        Ok(())
    }
    /** creating metaplex metadata accounts */
    pub fn mint_nft(ctx: Context<CreateMint>, _metadata_bump: u8, game_creator_bump: u8) -> ProgramResult {
        let metadata_infos = vec![
            ctx.accounts.metadata.to_account_info(),
            ctx.accounts.unchecked_nft_mint.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            ctx.accounts.game_creator_auth.to_account_info(),
        ];
        let creators: Vec<mpl_token_metadata::state::Creator> =
            vec![mpl_token_metadata::state::Creator {
                address: ctx.accounts.game_creator_pda.key(),
                verified: true,
                share: 100,
            }];
        let config_line = ConfigLine {
            uri: format!("https://42arbl3cd7lx2f3mmagvl2uraedjag7me2pslu7wuvhvhe3pmsjq.arweave.net/5oEQr2If130XbGANVeqRAQaQG-wmnyXT9qVPU5NvZJM/"),
            name: format!("dummy config")
        };
        let symbol = format!("$MON");
        let seller_basis_points = 650;
        let is_mutable = true;
        let seeds = &[PREFIX, &ctx.accounts.game_creator_auth.key().to_bytes()[..], CREATOR, &[game_creator_bump]];

        invoke_signed(
            &create_metadata_accounts(
                *ctx.accounts.token_metadata_program.key,
                *ctx.accounts.metadata.key,
                *ctx.accounts.unchecked_nft_mint.key,
                *ctx.accounts.mint_authority.key,
                *ctx.accounts.payer.key,
                *ctx.accounts.update_authority.key,
                config_line.name,
                symbol,
                config_line.uri,
                Some(creators),
                seller_basis_points,
                true,
                is_mutable
            ),
            metadata_infos.as_slice(),
            &[&seeds[..]],
        )?;
        
        sol_log_compute_units();

        let instruction_sysvar_account = &ctx.accounts.instruction_sysvar_account;
        let instruction_sysvar_account_info = instruction_sysvar_account.to_account_info();

        if let Err(e) = instruction_check(instruction_sysvar_account_info, ctx.program_id.key()) {
            return Err(e);
        }
        sol_log_compute_units();
        Ok(())
    }
    
    /** level-up mon, increasing its statistics as well as assigning new stat tokens for the corresponding increases */
    pub fn level_up_mon(ctx: Context<LevelUpMon>, _mon_state_bump: u8, game_creator_bump: u8, _mon_token_bump: u8, _mass_mint_bump: u8, _energy_mint_bump: u8, _order_mint_bump: u8) -> ProgramResult {
        let mon_state = &mut ctx.accounts.mon_state;
        let level = mon_state.level;
        let prev_stats: MonStatistics = mon_state.generate_stats(level);
        let next_stats: MonStatistics = mon_state.generate_stats(level + 1);
        let stats_added = MonStatistics::diff_stats(next_stats, prev_stats).to_mon_lamports();
        msg!("user_balance mass {}, energy {}, order {}", ctx.accounts.user_mass_ata_pda.amount, ctx.accounts.user_energy_ata_pda.amount, ctx.accounts.user_order_ata_pda.amount);
        msg!("stats_added mass {}, energy {}, order {}", stats_added.mass, stats_added.energy, stats_added.order);
        let stat_cost_ratio = TokenCalc::cost_to_level(
            MonStatistics { mass: ctx.accounts.mass_mint_pda.supply, energy: ctx.accounts.energy_mint_pda.supply, order: ctx.accounts.order_mint_pda.supply },
            ctx.accounts.mon_token_mint_pda.supply
        );
        msg!("stat_cost_ratio mass {}, energy {}, order {}", stat_cost_ratio.mass, stat_cost_ratio.energy, stat_cost_ratio.order);
        let stat_cost: MonStatistics = MonFloatStatistics::mul_stats_by_stats(stats_added.into(), stat_cost_ratio).into();
        msg!("stat_cost mass {}, energy {}, order {}", stat_cost.mass, stat_cost.energy, stat_cost.order);
        ctx.accounts.mass_mint_pda.supply;
        if stat_cost.mass > ctx.accounts.user_mass_ata_pda.amount {
            msg!("Insufficient mass tokens to level up!");
            return Err(ErrorCode::InsufficientMassTokens.into());
        } else if stat_cost.energy > ctx.accounts.user_energy_ata_pda.amount {
            msg!("Insufficient energy tokens to level up!");
            return Err(ErrorCode::InsufficientEnergyTokens.into());
        } else if stat_cost.order > ctx.accounts.user_order_ata_pda.amount {
            msg!("Insufficient order tokens to level up!");
            return Err(ErrorCode::InsufficientOrderTokens.into());
        }
        let seeds = &[PREFIX, &ctx.accounts.game_creator_auth.key().to_bytes()[..], CREATOR, &[game_creator_bump]];
        let signer = &[&seeds[..]];
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mass_mint_pda.to_account_info(),
                to: ctx.accounts.user_mass_ata_pda.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), stat_cost.mass)?;
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.energy_mint_pda.to_account_info(),
                to: ctx.accounts.user_energy_ata_pda.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), stat_cost.energy)?;
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.order_mint_pda.to_account_info(),
                to: ctx.accounts.user_order_ata_pda.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), stat_cost.order)?;
        mon_state.level += 1;
        Ok(())
    }
    
    /** destroy nft and give owning user its stat-tokens */
    pub fn dust_mon(ctx: Context<DustMon>, _mon_state_bump: u8, game_creator_bump: u8, _mass_mint_bump: u8, _energy_mint_bump: u8,_order_mint_bump: u8) -> ProgramResult {
        let mon_stat = &mut ctx.accounts.mon_state;
        let level = mon_stat.level.clone();
        let stats: MonStatistics = ctx.accounts.mon_state.generate_stats(level).to_mon_lamports();
        let seeds = &[PREFIX, &ctx.accounts.game_creator_auth.key().to_bytes()[..], CREATOR, &[game_creator_bump]];
        let signer = &[&seeds[..]];
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mass_mint_pda.to_account_info(),
                to: ctx.accounts.user_mass_ata_pda.to_account_info(),
                authority: ctx.accounts.game_creator_pda.to_account_info(),
            },
            signer
        ), stats.mass)?;
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.energy_mint_pda.to_account_info(),
                to: ctx.accounts.user_energy_ata_pda.to_account_info(),
                authority: ctx.accounts.game_creator_pda.to_account_info(),
            },
            signer
        ), stats.energy)?;
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.order_mint_pda.to_account_info(),
                to: ctx.accounts.user_order_ata_pda.to_account_info(),
                authority: ctx.accounts.game_creator_pda.to_account_info(),
            },
            signer
        ), stats.order)?;
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.nft_mint_pda.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), ctx.accounts.user_token_account.amount)?;
        token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.user_token_account.to_account_info(),
                destination: ctx.accounts.payer.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ))?;
        Ok(())
    }
    
    /** destroy stat tokens in exchange for main game token ($MON) */
    pub fn buy_mon_tokens(ctx: Context<BuyMonTokens>, amount: Option<u64>, game_creator_bump: u8, _mon_token_mint_bump: u8, _mass_mint_bump: u8, _energy_mint_bump: u8, _order_mint_bump: u8) -> ProgramResult {
        let TokenCalc { amount_stat_tokens, amount_mon_tokens } = TokenCalc::mon_tokens_user_can_buy(
            amount,
            MonStatistics { mass: ctx.accounts.user_mass_ata_pda.amount, energy: ctx.accounts.user_energy_ata_pda.amount, order: ctx.accounts.user_order_ata_pda.amount },
            MonStatistics { mass: ctx.accounts.mass_mint_pda.supply, energy: ctx.accounts.energy_mint_pda.supply, order: ctx.accounts.order_mint_pda.supply },
            ctx.accounts.mon_token_mint_pda.supply
        )?;
        msg!("amount_stat_tokens {}, amount_mon_tokens {}", amount_stat_tokens, amount_mon_tokens);
        let seeds = &[PREFIX, &ctx.accounts.game_creator_auth.key().to_bytes()[..], CREATOR, &[game_creator_bump]];
        let signer = &[&seeds[..]];
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.mass_mint_pda.to_account_info(),
                to: ctx.accounts.user_mass_ata_pda.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), amount_stat_tokens)?;
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.energy_mint_pda.to_account_info(),
                to: ctx.accounts.user_energy_ata_pda.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), amount_stat_tokens)?;
        token::burn(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.order_mint_pda.to_account_info(),
                to: ctx.accounts.user_order_ata_pda.to_account_info(),
                authority: ctx.accounts.user_account.to_account_info()
            },
            signer
        ), amount_stat_tokens)?;
        token::mint_to(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mon_token_mint_pda.to_account_info(),
                to: ctx.accounts.user_mon_token_ata_pda.to_account_info(),
                authority: ctx.accounts.game_creator_pda.to_account_info(),
            },
            signer
        ), amount_mon_tokens)?;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(game_creator_bump: u8, mon_token_bump: u8, mass_mint_bump: u8, energy_mint_bump: u8, order_mint_bump: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(mut, address = game_creator_auth.key())]
    payer: Signer<'info>,
    #[account(mut)]
    mon_token_mint_key: Signer<'info>,
    #[account(mut)]
    mass_mint_key: Signer<'info>,
    #[account(mut)]
    energy_mint_key: Signer<'info>,
    #[account(mut)]
    order_mint_key: Signer<'info>,
    #[account(init, mint::decimals = DECIMALS, mint::authority = game_creator_pda, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mon_token_mint_key.key().to_bytes()[..], MON_TOKEN_SEED], bump = mon_token_bump, payer = payer)]
    mon_token_mint_pda: Account<'info, Mint>,
    #[account(init, mint::decimals = DECIMALS, mint::authority = game_creator_pda, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mass_mint_key.key().to_bytes()[..], MASS_SEED], bump = mass_mint_bump, payer = payer)]
    mass_mint_pda: Account<'info, Mint>,
    #[account(init, mint::decimals = DECIMALS, mint::authority = game_creator_pda, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &energy_mint_key.key().to_bytes()[..], ENERGY_SEED], bump = energy_mint_bump, payer = payer)]
    energy_mint_pda: Account<'info, Mint>,
    #[account(init, mint::decimals = DECIMALS, mint::authority = game_creator_pda, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &order_mint_key.key().to_bytes()[..], ORDER_SEED], bump = order_mint_bump, payer = payer)]
    order_mint_pda: Account<'info, Mint>,
    // #[account(init, associated_token::mint = mon_token_mint_pda, associated_token::authority = game_creator_pda, payer = payer)]
    // treasury_mon_token_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(init, associated_token::mint = mass_mint_pda, associated_token::authority = game_creator_pda, payer = payer)]
    // treasury_mass_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(init, associated_token::mint = energy_mint_pda, associated_token::authority = game_creator_pda, payer = payer)]
    // treasury_energy_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(init, associated_token::mint = order_mint_pda, associated_token::authority = game_creator_pda, payer = payer)]
    // treasury_order_ata_pda: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    associated_token_program: Program<'info, AssociatedToken>,
}
#[derive(Accounts)]
#[instruction(amount: u64, stat_seed: String, mint_bump: u8, game_creator_bump: u8)]
pub struct MintStatTokensTo<'info> {
    #[account(mut)]
    stat_mint_key: AccountInfo<'info>,
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut, address = game_creator_auth.key())]
    payer: Signer<'info>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &stat_mint_key.key().to_bytes()[..], &stat_seed.as_bytes()], bump = mint_bump)]
    stat_mint_pda: Box<Account<'info, Mint>>,
    #[account(init_if_needed, associated_token::mint = stat_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_stat_ata_pda: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    associated_token_program: Program<'info, AssociatedToken>,
    rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
#[instruction(game_creator_bump: u8, mon_token_bump: u8, mass_mint_bump: u8, energy_mint_bump: u8, order_mint_bump: u8)]
pub struct InitializeUser<'info> {
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut)]
    mon_token_mint_key: AccountInfo<'info>,
    #[account(mut)]
    mass_mint_key: AccountInfo<'info>,
    #[account(mut)]
    energy_mint_key: AccountInfo<'info>,
    #[account(mut)]
    order_mint_key: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mon_token_mint_key.key().to_bytes()[..], MON_TOKEN_SEED], bump = mon_token_bump)]
    mon_token_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mass_mint_key.key().to_bytes()[..], MASS_SEED], bump = mass_mint_bump)]
    mass_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &energy_mint_key.key().to_bytes()[..], ENERGY_SEED], bump = energy_mint_bump)]
    energy_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &order_mint_key.key().to_bytes()[..], ORDER_SEED], bump = order_mint_bump)]
    order_mint_pda: Box<Account<'info, Mint>>,
    #[account(init_if_needed, associated_token::mint = mon_token_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_mon_token_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = mass_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_mass_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = energy_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_energy_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = order_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_order_ata_pda: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    associated_token_program: Program<'info, AssociatedToken>,
}
#[derive(Accounts)]
#[instruction(game_creator_bump: u8, nft_mint_bump: u8)]
pub struct InitializeNftMint<'info> {
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    nft_mint_key: AccountInfo<'info>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(init, mint::decimals = 0, mint::authority = game_creator_pda, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &nft_mint_key.key().to_bytes()[..]], bump = nft_mint_bump, payer = payer)]
    nft_mint_pda: Box<Account<'info, Mint>>,
    #[account(init, associated_token::mint = nft_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_token_account: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    associated_token_program: Program<'info, AssociatedToken>,
}
#[derive(Accounts)]
#[instruction(mon_state_bump: u8)]
pub struct InitializeNftMonState<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    nft_mint: AccountInfo<'info>,
    #[account(init, space=std::mem::size_of::<MonState>() + 8, seeds = [MON_STATE_SEED, &nft_mint.key().to_bytes()[..]], bump = mon_state_bump, payer = payer)]
    mon_state: Account<'info, MonState>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
#[instruction(metadata_bump: u8, game_creator_bump: u8)]
pub struct CreateMint<'info> {
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut, associated_token::mint = nft_mint_pda, associated_token::authority = user_account)]
    user_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    metadata: UncheckedAccount<'info>,
    #[account(mut)]
    unchecked_nft_mint: UncheckedAccount<'info>,
    #[account(mut)]
    nft_mint_pda: AccountInfo<'info>,
    #[account(mut)]
    mint_authority: AccountInfo<'info>,
    #[account(mut)]
    update_authority: AccountInfo<'info>,
    #[account(mut)]
    master_edition: UncheckedAccount<'info>,
    #[account(address = mpl_token_metadata::id())]
    token_metadata_program: UncheckedAccount<'info>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    clock: Sysvar<'info, Clock>,
    #[account(address = sysvar::recent_blockhashes::id())]
    recent_blockhashes: UncheckedAccount<'info>,
    #[account(address = sysvar::instructions::id())]
    instruction_sysvar_account: UncheckedAccount<'info>,
}
#[derive(Accounts)]
#[instruction(mon_state_bump: u8, game_creator_bump: u8, mon_token_mint_bump: u8, mass_mint_bump: u8, energy_mint_bump: u8, order_mint_bump: u8)]
pub struct LevelUpMon<'info> {
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut)]
    mon_token_mint_key: AccountInfo<'info>,
    #[account(mut)]
    mass_mint_key: AccountInfo<'info>,
    #[account(mut)]
    energy_mint_key: AccountInfo<'info>,
    #[account(mut)]
    order_mint_key: AccountInfo<'info>,
    #[account(mut, associated_token::mint = nft_mint_pda, associated_token::authority = user_account)]
    user_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(mut)]
    nft_mint_pda: AccountInfo<'info>,
    #[account(mut, seeds = [MON_STATE_SEED, &nft_mint_pda.key().to_bytes()[..]], bump = mon_state_bump,)]
    mon_state: Account<'info, MonState>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mon_token_mint_key.key().to_bytes()[..], MON_TOKEN_SEED], bump = mon_token_mint_bump)]
    mon_token_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mass_mint_key.key().to_bytes()[..], MASS_SEED], bump = mass_mint_bump)]
    mass_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &energy_mint_key.key().to_bytes()[..], ENERGY_SEED], bump = energy_mint_bump)]
    energy_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &order_mint_key.key().to_bytes()[..], ORDER_SEED], bump = order_mint_bump)]
    order_mint_pda: Box<Account<'info, Mint>>,
    #[account(init_if_needed, associated_token::mint = mass_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_mass_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = energy_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_energy_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = order_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_order_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(mut, associated_token::mint = mon_token_mint_pda, associated_token::authority = game_creator_pda)]
    // treasury_mon_token_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(mut, associated_token::mint = mass_mint_pda, associated_token::authority = game_creator_pda)]
    // treasury_mass_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(mut, associated_token::mint = energy_mint_pda, associated_token::authority = game_creator_pda)]
    // treasury_energy_ata_pda: Box<Account<'info, TokenAccount>>,
    // #[account(mut, associated_token::mint = order_mint_pda, associated_token::authority = game_creator_pda)]
    // treasury_order_ata_pda: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    associated_token_program: Program<'info, AssociatedToken>,
}
#[derive(Accounts)]
#[instruction(mon_state_bump: u8, game_creator_bump: u8, mass_mint_bump: u8, energy_mint_bump: u8, order_mint_bump: u8)]
pub struct DustMon<'info> {
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut)]
    mass_mint_key: AccountInfo<'info>,
    #[account(mut)]
    energy_mint_key: AccountInfo<'info>,
    #[account(mut)]
    order_mint_key: AccountInfo<'info>,
    #[account(mut, associated_token::mint = nft_mint_pda, associated_token::authority = user_account)]
    user_token_account: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(mut)]
    nft_mint_pda: Account<'info, Mint>,
    #[account(mut, seeds = [MON_STATE_SEED, &nft_mint_pda.key().to_bytes()[..]], bump = mon_state_bump,)]
    mon_state: Account<'info, MonState>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mass_mint_key.key().to_bytes()[..], MASS_SEED], bump = mass_mint_bump)]
    mass_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &energy_mint_key.key().to_bytes()[..], ENERGY_SEED], bump = energy_mint_bump)]
    energy_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &order_mint_key.key().to_bytes()[..], ORDER_SEED], bump = order_mint_bump)]
    order_mint_pda: Box<Account<'info, Mint>>,
    #[account(init_if_needed, associated_token::mint = mass_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_mass_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = energy_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_energy_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = order_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_order_ata_pda: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    associated_token_program: Program<'info, AssociatedToken>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}
#[derive(Accounts)]
#[instruction(amount: Option<u64>, game_creator_bump: u8, mon_token_mint_bump: u8, mass_mint_bump: u8, energy_mint_bump: u8, order_mint_bump: u8)]
pub struct BuyMonTokens<'info> {
    #[account(mut)]
    user_account: AccountInfo<'info>,
    #[account(mut)]
    mon_token_mint_key: AccountInfo<'info>,
    #[account(mut)]
    mass_mint_key: AccountInfo<'info>,
    #[account(mut)]
    energy_mint_key: AccountInfo<'info>,
    #[account(mut)]
    order_mint_key: AccountInfo<'info>,
    #[account(mut)]
    payer: Signer<'info>,
    #[account(mut)]
    game_creator_auth: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], CREATOR], bump = game_creator_bump)]
    game_creator_pda: AccountInfo<'info>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mon_token_mint_key.key().to_bytes()[..], MON_TOKEN_SEED], bump = mon_token_mint_bump)]
    mon_token_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &mass_mint_key.key().to_bytes()[..], MASS_SEED], bump = mass_mint_bump)]
    mass_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &energy_mint_key.key().to_bytes()[..], ENERGY_SEED], bump = energy_mint_bump)]
    energy_mint_pda: Box<Account<'info, Mint>>,
    #[account(mut, seeds = [PREFIX, &game_creator_auth.key().to_bytes()[..], &order_mint_key.key().to_bytes()[..], ORDER_SEED], bump = order_mint_bump)]
    order_mint_pda: Box<Account<'info, Mint>>,
    #[account(init_if_needed, associated_token::mint = mon_token_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_mon_token_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = mass_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_mass_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = energy_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_energy_ata_pda: Box<Account<'info, TokenAccount>>,
    #[account(init_if_needed, associated_token::mint = order_mint_pda, associated_token::authority = user_account, payer = payer,)]
    user_order_ata_pda: Box<Account<'info, TokenAccount>>,
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
    associated_token_program: Program<'info, AssociatedToken>,
}

#[account]
#[derive(Default)]
pub struct MonState {
    seed: u64,
    level: u64
}

#[derive(Default, Clone, Copy)]
pub struct MonStatistics {
    mass: u64,
    energy: u64,
    order: u64
}

#[derive(Default, Clone, Copy)]
pub struct MonFloatStatistics {
    mass: f64,
    energy: f64,
    order: f64
}

impl MonFloatStatistics {
    pub fn mul_stats_by_val(mul_this: MonFloatStatistics, by_this: f64) -> MonFloatStatistics {
        MonFloatStatistics {
            mass: mul_this.mass * by_this,
            energy: mul_this.energy * by_this,
            order: mul_this.order * by_this
        }
    }
    pub fn mul_stats_by_stats(mul_this: MonFloatStatistics, by_this: MonFloatStatistics) -> MonFloatStatistics {
        MonFloatStatistics {
            mass: mul_this.mass * by_this.mass,
            energy: mul_this.energy * by_this.energy,
            order: mul_this.order * by_this.order
        }
    }
    pub fn div_stats_by_val(div_this: MonFloatStatistics, by_this: f64) -> MonFloatStatistics {
        MonFloatStatistics {
            mass: div_this.mass / by_this,
            energy: div_this.energy / by_this,
            order: div_this.order / by_this
        }
    }
    pub fn diff_stats_by_stats(from_this: MonFloatStatistics, take_this: MonFloatStatistics) -> MonFloatStatistics {
        MonFloatStatistics {
            mass: from_this.mass - take_this.mass,
            energy: from_this.energy - take_this.energy,
            order: from_this.order - take_this.order
        }
    }
}

impl From<MonFloatStatistics> for MonStatistics {
    fn from(stats: MonFloatStatistics) -> Self {
        MonStatistics { mass: stats.mass as u64, energy: stats.energy as u64, order: stats.order as u64 }
    }
}

impl From<MonStatistics> for MonFloatStatistics {
    fn from(stats: MonStatistics) -> Self {
        MonFloatStatistics { mass: stats.mass as f64, energy: stats.energy as f64, order: stats.order as f64 }
    }
}

impl MonStatistics {
    pub fn to_mon_lamports(&self) -> MonStatistics {
        MonStatistics {
            mass: self.mass * 10_u64.pow(DECIMALS as u32),
            energy: self.energy * 10_u64.pow(DECIMALS as u32),
            order: self.order * 10_u64.pow(DECIMALS as u32)
        }
    }
    pub fn sum_parts(&self) -> u64 {
        self.mass + self.energy + self.order
    }
    pub fn diff_stats(from_this: MonStatistics, take_this: MonStatistics) -> MonStatistics {
        MonStatistics {
            mass: from_this.mass - take_this.mass,
            energy: from_this.energy - take_this.energy,
            order: from_this.order - take_this.order
        }
    }
}

impl MonState {
    pub fn generate_stats(&self, level: u64) -> MonStatistics {
        MonStatistics {
            mass: roll_stat_per_level(self.seed, level),
            energy: roll_stat_per_level(self.seed - 1, level),
            order: roll_stat_per_level(self.seed - 2, level)
        }
    }
}


pub struct TokenCalc {
    amount_stat_tokens: u64,
    amount_mon_tokens: u64
}

impl TokenCalc {
    pub fn mon_tokens_user_can_buy(amount: Option<u64>, user_balance: MonStatistics, stat_token_supply: MonStatistics, mon_token_supply: u64) -> Result<TokenCalc, ErrorCode> {
        // msg!("stat_token_supply mass {}, energy {}, order {}, mon_token_supply {}", stat_token_supply.mass, stat_token_supply.energy, stat_token_supply.order, mon_token_supply);
        // msg!("user_balance mass {}, energy {}, order {}, mon_token_supply {}", user_balance.mass, user_balance.energy, user_balance.order, mon_token_supply);
        let summed_stat_tokens = stat_token_supply.sum_parts();
        let mut lowest = user_balance.mass;
        if user_balance.energy < lowest {
            lowest = user_balance.energy;
        }
        if user_balance.order < lowest {
            lowest = user_balance.order;
        }
        let amount = if let Some(amount) = amount {
            amount
        } else {
            lowest
        };
        if amount > lowest {
            return Err(ErrorCode::InvalidMonTokenAmount);
        }
        let mon_token_per_stat_token = mon_token_supply as f64 / summed_stat_tokens as f64;
        let amount_mon_tokens = ((amount as f64) * mon_token_per_stat_token) as u64;
        Ok(TokenCalc { amount_stat_tokens: amount, amount_mon_tokens })
        // let amount_requested = ((amount as f64) * mon_token_per_stat_token) as u64;
        // let amount_requested = if amount_requested > lowest {
        //     lowest
        // } else {
        //     amount_requested
        // };
        // let is_mon_token_higher = mon_token_per_stat_token > 1.;
        // if !is_mon_token_higher {
        //     /** You always need 1 of each stat token to get 1 mon token **/
        //     let max_can_buy = ((lowest as f64) * mon_token_per_stat_token) as u64;
        //     let amount_stat_tokens = if let Some(amount) = amount {
        //         ((amount as f64) * mon_token_per_stat_token) as u64
        //     } else {
        //         max_can_buy
        //     };
        //     if amount_stat_tokens > max_can_buy {
        //         return Err(ErrorCode::InvalidMonTokenAmount);
        //     }
        //     let amount_mon_tokens = if let Some(amount) = amount {
        //         amount
        //     } else {
        //         lowest
        //     };
        //     Ok(TokenCalc { amount_stat_tokens, amount_mon_tokens })
        // } else {
        //     /** You always need 1 of each stat token to get 1 mon token **/
        //     // let max_can_buy = lowest;
        //     let amount_mon_tokens = if let Some(amount) = amount {
        //         ((amount as f64) * mon_token_per_stat_token) as u64
        //     } else {
        //         lowest
        //     };
        //     // if amount_stat_tokens > lowest {
        //     //     return Err(ErrorCode::InvalidMonTokenAmount);
        //     // }
        //     let amount_stat_tokens = if let Some(amount) = amount {
        //         amount
        //     } else {
        //         lowest
        //     };
        //     Ok(TokenCalc { amount_stat_tokens, amount_mon_tokens })
        // }

        // msg!("lowest {}, mon_token_per_stat_token {}, res {}, summed_stat_tokens {}", lowest, mon_token_per_stat_token, res, summed_stat_tokens);
        // res
    }
    pub fn cost_to_level(stat_token_supply: MonStatistics, mon_token_supply: u64) -> MonFloatStatistics {
        msg!("stat_token_supply mass {}, energy {}, order {}, mon_token_supply {}", stat_token_supply.mass, stat_token_supply.energy, stat_token_supply.order, mon_token_supply);
        // msg!("by_this mass {}, energy {}, entropy {}", by_this.mass, by_this.energy, by_this.order);
        let summed_stat_tokens = stat_token_supply.sum_parts();
        // msg!("stat_token_supply mass {}, energy {}, entropy {}, mon_token_supply {}, summed_stat_tokens {}, denom {}", stat_token_supply.mass, stat_token_supply.energy, stat_token_supply.order, mon_token_supply, summed_stat_tokens, (summed_stat_tokens * mon_token_supply) as f64);
        // let token_ratios = MonFloatStatistics::div_stats_by_val(stat_token_supply.into(), (summed_stat_tokens * mon_token_supply) as f64);
        /**  Get ratio of each token to the total of all three **/
        let token_ratios = MonFloatStatistics::div_stats_by_val(stat_token_supply.into(), summed_stat_tokens as f64);
        /** Divide this ratio by the total supply of mon tokens so that supply of stat tokens increases as it gets closer to the supply of mon tokens **/
        let stat_per_mon_ratio = 1_f64 / (summed_stat_tokens as f64 / mon_token_supply as f64);
        MonFloatStatistics::mul_stats_by_val(token_ratios, stat_per_mon_ratio)
        // MonFloatStatistics::cube_root_stats(MonFloatStatistics::mul_stats_by_val(token_ratios, mon_token_to_stat_ratio.powf(3.)))
        // msg!("token_ratios mass {}, energy {}, entropy {}", token_ratios.mass, token_ratios.energy, token_ratios.order);
        // MonFloatStatistics::diff_stats_by_stats(MonFloatStatistics { mass: 1., energy: 1., order: 1. }, token_ratios);
        // msg!("stat_cost_ratio mass {}, energy {}, order {}", stat_cost_ratio.mass, stat_cost_ratio.energy, stat_cost_ratio.order);
        // let stat_cost: MonStatistics = MonFloatStatistics::mul_stats_by_stats(stats_added.into(), stat_cost_ratio).into();
    }
}
// #[account]
// pub struct Metadata {
//     data: Data
// }
//
// #[derive(AnchorSerialize, AnchorDeserialize, Default, Clone)]
// pub struct Data {
//     crypto_mon_seed: u8,
//     experience: u8
// }

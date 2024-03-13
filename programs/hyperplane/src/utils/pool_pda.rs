use anchor_lang::{
    prelude::{AccountInfo, CpiContext, Pubkey, Rent, SolanaSysvar},
    Result, ToAccountInfo,
};
use anchor_spl::token::{Mint, TokenAccount};

#[allow(clippy::too_many_arguments)]
pub fn create_pool_token_account<'info>(
    token_program: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    admin: &AccountInfo<'info>,
    pool: &AccountInfo<'info>,
    token_mint: &AccountInfo<'info>,
    token_account: &AccountInfo<'info>,
    token_account_derive_fct: fn(&Pubkey, &Pubkey, &Pubkey) -> (Pubkey, u8),
    token_account_seed: &[u8],
    pool_authority: &AccountInfo<'info>,
) -> Result<()> {
    let token_account_bump = token_account_derive_fct(&crate::ID, pool.key, token_mint.key).1;
    let token_account_seeds = [
        token_account_seed,
        pool.key.as_ref(),
        token_mint.key.as_ref(),
        &[token_account_bump],
    ];
    let signer_seeds = &[&token_account_seeds[..]];
    let extension_size = token_mint.data_len().saturating_sub(Mint::LEN);
    let account_size = TokenAccount::LEN + extension_size;
    let lamports = Rent::get()?.minimum_balance(account_size);
    anchor_lang::system_program::create_account(
        CpiContext::new_with_signer(
            system_program.clone(),
            anchor_lang::system_program::CreateAccount {
                from: admin.clone(),
                to: token_account.clone(),
            },
            signer_seeds,
        ),
        lamports,
        account_size as u64,
        token_program.key,
    )?;
    anchor_spl::token_2022::initialize_account3(CpiContext::new_with_signer(
        token_program.clone(),
        anchor_spl::token_2022::InitializeAccount3 {
            mint: token_mint.to_account_info(),
            authority: pool_authority.clone(),
            account: token_account.clone(),
        },
        signer_seeds,
    ))?;

    Ok(())
}

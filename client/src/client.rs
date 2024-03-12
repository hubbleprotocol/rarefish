use anchor_client::{
    anchor_lang::{prelude::Pubkey, system_program::System, AccountDeserialize, Id},
    solana_sdk::{
        rent::Rent,
        signature::{Keypair, Signer},
        sysvar::SysvarId,
    },
};
use anchor_spl::{associated_token::AssociatedToken, token::TokenAccount};
use anyhow::Result;
use hyperplane::{
    ix::{Initialize, UpdatePoolConfig},
    state::SwapPool,
    utils::seeds::{pda, pda::InitPoolPdas},
    InitialSupply,
};
use orbit_link::{async_client::AsyncClient, OrbitLink};
use tracing::info;

use crate::send_tx;

pub struct HyperplaneClient<T: AsyncClient, S: Signer> {
    pub client: OrbitLink<T, S>,
    pub config: Config,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Config {
    /// Hyperplane program id
    pub program_id: Pubkey,
    /// Send the transaction without actually executing it
    pub dry_run: bool,
    /// Encode the transaction in base58 and base64 and print it to stdout
    /// Instructions which require private key signer (e.g. zero-copy account allocations) will not executed immediately
    pub multisig: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            program_id: hyperplane::ID,
            dry_run: false,
            multisig: false,
        }
    }
}

impl<T, S> HyperplaneClient<T, S>
where
    T: AsyncClient,
    S: Signer,
{
    #[tracing::instrument(skip(client))] //Skip client that does not impl Debug
    pub async fn new(client: OrbitLink<T, S>, config: Config) -> Result<Self> {
        Ok(Self { client, config })
    }

    pub async fn initialize_pool(
        &self,
        admin: Pubkey,
        admin_token_a_ata: Pubkey,
        admin_token_b_ata: Pubkey,
        Initialize {
            fees,
            curve_parameters,
            initial_supply:
                InitialSupply {
                    initial_supply_a,
                    initial_supply_b,
                },
        }: Initialize,
    ) -> Result<Pubkey> {
        let pool_kp = Keypair::new();

        info!("Pool: {}", pool_kp.pubkey());

        let a_ata = self.client.client.get_account(&admin_token_a_ata).await?;
        let token_a_token_program = a_ata.owner;
        let mut a_ata_data: &[u8] = &a_ata.data;
        let token_a_mint = TokenAccount::try_deserialize(&mut a_ata_data)?.mint;
        info!("Token A mint: {}", token_a_mint);
        info!("Token A token program: {}", token_a_token_program);

        let b_ata = self.client.client.get_account(&admin_token_b_ata).await?;
        let token_b_token_program = b_ata.owner;
        let mut b_ata_data: &[u8] = &b_ata.data;
        let token_b_mint = TokenAccount::try_deserialize(&mut b_ata_data)?.mint;
        info!("Token B mint: {}", token_b_mint);
        info!("Token B token program: {}", token_b_token_program);

        let InitPoolPdas {
            curve,
            authority,
            token_a_vault,
            token_b_vault,
            pool_token_mint,
            token_a_fees_vault,
            token_b_fees_vault,
        } = pda::init_pool_pdas_program_id(
            &self.config.program_id,
            &pool_kp.pubkey(),
            &token_a_mint,
            &token_b_mint,
        );
        let admin_pool_token_ata =
            anchor_spl::associated_token::get_associated_token_address(&admin, &pool_token_mint);
        info!("Admin pool token ATA: {}", admin_pool_token_ata);

        let mut tx = self.client.tx_builder().add_ix(
            // Account for the swap pool, zero copy
            self.client
                .create_account_ix(&pool_kp.pubkey(), SwapPool::LEN, &self.config.program_id)
                .await?,
        );

        let pool_token_program = spl_token::id();
        // let (token_a_vault, _token_a_vault_bump_seed) =
        //     hyperplane::utils::seeds::pda::token_a_vault_pda(&pool_kp.pubkey(), &token_a_mint);
        // tx = tx.add_ix(
        //     self.client
        //         .create_account_ix(
        //             &token_a_vault,
        //             TokenAccount::LEN,
        //             &token_a_token_program,
        //         )
        //         .await?,
        // );
        // tx = tx.add_ix(
        //     spl_token::instruction::initialize_account3(
        //         &token_a_token_program,
        //         &token_a_vault,
        //         &token_a_mint,
        //         &authority,
        //     )
        //     .unwrap(),
        // );

        if self.config.multisig {
            // Allocate space and assign to token program for the admin pool token account
            // This is required because multisig does not support additional signers
            // Cannot fully init the token account as the mint does not exist yet
            // tx = tx.add_ix(
            //     self.client
            //         .create_account_ix(
            //             &admin_pool_token_ata.pubkey(),
            //             TokenAccount::LEN,
            //             &pool_token_program,
            //         )
            //         .await?,
            // );
            // info!(
            //     "Sending non-multisig txs to allocate for space pool account: {} and admin pool token ATA: {}",
            //     pool_kp.pubkey(),
            //     admin_pool_token_ata.pubkey()
            // );
            // send_tx!(self, tx, [&pool_kp, &admin_pool_token_ata]);
            // tx = self.client.tx_builder();
        }

        tx = tx.add_anchor_ix(
            &self.config.program_id,
            hyperplane::accounts::InitializePool {
                admin,
                pool: pool_kp.pubkey(),
                swap_curve: curve,
                pool_authority: authority,
                token_a_mint,
                token_b_mint,
                token_a_vault,
                token_b_vault,
                pool_token_mint,
                token_a_fees_vault,
                token_b_fees_vault,
                admin_token_a_ata,
                admin_token_b_ata,
                admin_pool_token_ata,
                system_program: System::id(),
                rent: Rent::id(),
                pool_token_program,
                token_a_token_program,
                token_b_token_program,
                associated_token_program: AssociatedToken::id(),
            },
            hyperplane::instruction::InitializePool {
                initial_supply_a,
                initial_supply_b,
                fees,
                curve_parameters,
            },
        );

        if self.config.multisig {
            send_tx!(self, tx, []);
        } else {
            send_tx!(self, tx, [&pool_kp]);
        }

        Ok(pool_kp.pubkey())
    }

    pub async fn update_pool_config(
        &self,
        admin: Pubkey,
        pool: Pubkey,
        update: UpdatePoolConfig,
    ) -> Result<()> {
        // let swap_pool: SwapPool = self.client.get_anchor_account(&pool).await?;
        let tx = self.client.tx_builder().add_anchor_ix(
            &self.config.program_id,
            hyperplane::accounts::UpdatePoolConfig { admin, pool },
            hyperplane::instruction::UpdatePoolConfig::from(update),
        );
        send_tx!(self, tx, []);

        Ok(())
    }

    /// Get an the rpc instance used by the KLendClient
    pub fn get_rpc(&self) -> &T {
        &self.client.client
    }
}

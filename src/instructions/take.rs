use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::state::Escrow;

pub fn process_take_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [taker, maker, mint_a, mint_b, escrow_account, taker_ata_a, taker_ata_b, maker_ata_b, escrow_ata, system_program, token_program, _associated_token_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    {
        let taker_ata_a_state =
            pinocchio_token::state::TokenAccount::from_account_view(&taker_ata_a)?;

        let taker_ata_b_state =
            pinocchio_token::state::TokenAccount::from_account_view(&taker_ata_b)?;

        let maker_ata_b_state =
            pinocchio_token::state::TokenAccount::from_account_view(&maker_ata_b)?;

        if taker_ata_a_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_a_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        if taker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        if maker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let bump = data[0];
    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let seeds = &seed[..];

    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    // let amount_to_receive = unsafe { *(data.as_ptr().add(1) as *const u64) };
    // let amount_to_give = unsafe { *(data.as_ptr().add(9) as *const u64) };

    let bump = [bump.to_le()];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);

    // unsafe {
    //     if escrow_account.owner() != &crate::ID {
    //         CreateAccount {
    //             from: maker,
    //             to: escrow_account,
    //             lamports: Rent::get()?.try_minimum_balance(Escrow::LEN)?,
    //             space: Escrow::LEN as u64,
    //             owner: &crate::ID,
    //         }
    //         .invoke_signed(&[seeds.clone()])?;

    //         {
    //             let escrow_state = Escrow::from_account_info(escrow_account)?;

    //             escrow_state.set_maker(maker.address());
    //             escrow_state.set_mint_a(mint_a.address());
    //             escrow_state.set_mint_b(mint_b.address());
    //             escrow_state.set_amount_to_receive(amount_to_receive);
    //             escrow_state.set_amount_to_give(amount_to_give);
    //             escrow_state.bump = data[0];
    //         }
    //     } else {
    //         return Err(ProgramError::IllegalOwner);
    //     }
    // }

    pinocchio_associated_token_account::instructions::Create {
        funding_account: taker,
        account: escrow_ata,
        wallet: escrow_account,
        mint: mint_b,
        token_program: token_program,
        system_program: system_program,
    }
    .invoke()?;

    pinocchio_token::instructions::Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: mint_b.lamports(),
    }
    .invoke_signed(&[seeds.clone()])?;

    Ok(())
}

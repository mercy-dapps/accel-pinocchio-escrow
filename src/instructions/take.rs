use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};
use pinocchio_pubkey::derive_address;

use crate::state::Escrow;

pub fn process_take_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    let [taker, maker, mint_a, mint_b, escrow_account, taker_ata_a, taker_ata_b, maker_ata_b, escrow_ata, _system_program, _token_program, _associated_token_program @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // taker ata a account checks
    {
        let taker_ata_a_state =
            pinocchio_token::state::TokenAccount::from_account_view(&taker_ata_a)?;

        if taker_ata_a_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_a_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // taker ata b account checks
    {
        let taker_ata_b_state =
            pinocchio_token::state::TokenAccount::from_account_view(&taker_ata_b)?;
        if taker_ata_b_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // maker ata account checks
    {
        let maker_ata_b_state =
            pinocchio_token::state::TokenAccount::from_account_view(&maker_ata_b)?;
        if maker_ata_b_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // escrow account checks
    {
        let escrow_account_state = Escrow::from_account_info(&escrow_account)?;

        if &escrow_account_state.maker() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }

        if &escrow_account_state.mint_a() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        if &escrow_account_state.mint_b() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // escrow ata account checks
    {
        let escrow_ata_state =
            pinocchio_token::state::TokenAccount::from_account_view(&escrow_ata)?;

        if escrow_ata_state.owner() != escrow_account.address() {
            return Err(ProgramError::IllegalOwner);
        }

        if escrow_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let bump = {
        let escrow_state = Escrow::from_account_info(escrow_account)?;
        escrow_state.bump
    };

    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];

    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    let (amount_to_receive, amount_to_give) = {
        let escrow_account_state = Escrow::from_account_info(&escrow_account)?;
        (escrow_account_state.amount_to_receive(), escrow_account_state.amount_to_give())
    };

    let bump = [bump.to_le()];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);

    pinocchio_token::instructions::Transfer {
        from: escrow_ata,
        to: taker_ata_a,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[seeds.clone()])?;
    
    pinocchio_token::instructions::Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive,
    }
    .invoke()?;

    pinocchio_token::instructions::CloseAccount {
        account: escrow_ata,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[seeds.clone()])?;

    Ok(())
}

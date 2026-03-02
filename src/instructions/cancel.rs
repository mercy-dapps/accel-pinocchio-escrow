use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};

use pinocchio_pubkey::derive_address;

use crate::state::Escrow;

pub fn process_cancel_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [maker, mint_a, escrow_account, maker_ata, escrow_ata, _token_program, _system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // maker ata for mint_a checks

    {
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_view(&maker_ata)?;

        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }

        if maker_ata_state.mint() != mint_a.address() {
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

    let bump = data[0];

    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];

    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());

    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    let amount_to_give = {
        let escrow_account_state = Escrow::from_account_info(&escrow_account)?;
        escrow_account_state.amount_to_give()
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
        to: maker_ata,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[seeds.clone()])?;

    pinocchio_token::instructions::CloseAccount {
        account: escrow_ata,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[seeds.clone()])?;

    Ok(())
}

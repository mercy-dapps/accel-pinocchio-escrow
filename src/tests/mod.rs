#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{
        spl_token::{self},
        CreateAssociatedTokenAccount, CreateMint, MintTo,
    };

    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    const PROGRAM_ID: &str = "4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT";
    const TOKEN_PROGRAM_ID: Pubkey = spl_token::ID;
    const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, Keypair, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        let taker = Keypair::new();

        // Load program SO file
        println!("The path is!! {}", env!("CARGO_MANIFEST_DIR"));
        let so_path = PathBuf::from(
            "/Users/mac/Desktop/cool/turbin3/accelerated cohort 26/accel-pinocchio-escrow/target/deploy/escrow.so",
        );

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        svm.add_program(program_id(), &program_data)
            .expect("Failed to add program");

        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        (svm, payer, taker)
    }

    fn helper(
        svm: &mut LiteSVM,
        payer: &Keypair,
        taker: &Keypair,
    ) -> (
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        Pubkey,
        u8,
    ) {
        let program_id = program_id();

        assert_eq!(program_id.to_string(), PROGRAM_ID);

        let mint_a = CreateMint::new(svm, payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        println!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(svm, taker)
            .decimals(6)
            .authority(&taker.pubkey())
            .send()
            .unwrap();
        println!("Mint B: {}", mint_b);

        // Create the maker's associated token account for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(svm, payer, &mint_a)
            .owner(&payer.pubkey())
            .send()
            .unwrap();
        println!("Maker ATA A: {}\n", maker_ata_a);

        let maker_ata_b = CreateAssociatedTokenAccount::new(svm, taker, &mint_b)
            .owner(&payer.pubkey())
            .send()
            .unwrap();
        println!("Maker ATA B: {}\n", maker_ata_b);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), payer.pubkey().as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        println!("Escrow PDA: {}\n", escrow.0);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = spl_associated_token_account::get_associated_token_address(
            &escrow.0, // owner will be the escrow PDA
            &mint_a,   // mint
        );
        println!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(svm, payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        let amount_to_receive: u64 = 100000000; // 100 tokens with 6 decimal places
        let amount_to_give: u64 = 500000000; // 500 tokens with 6 decimal places
        let bump: u8 = escrow.1;

        println!("Bump: {}", bump);

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_data = [
            vec![0u8], // Discriminator for "Make" instruction
            bump.to_le_bytes().to_vec(),
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ]
        .concat();
        let make_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_a, false),
                AccountMeta::new_readonly(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(system_program, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(associated_token_program, false),
            ],
            data: make_data,
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();

        let transaction = Transaction::new(&[payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = svm.send_transaction(transaction).unwrap();

        // Log transaction details
        println!("\n\nMake transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);

        (
            mint_a,
            mint_b,
            maker_ata_a,
            maker_ata_b,
            escrow.0,
            vault,
            associated_token_program,
            token_program,
            system_program,
            bump,
        )
    }

    #[test]
    pub fn test_take_instruction() {
        let (mut svm, payer, taker) = setup();
        let program_id = program_id();

        let (mint_a, mint_b, _, maker_ata_b, escrow, vault, _, token_program, _, bump) =
            helper(&mut svm, &payer, &taker);

        let take_data = [
            vec![1u8], // Discriminator for "Take" instruction
            bump.to_le_bytes().to_vec(),
        ]
        .concat();

        // Create the maker's associated token account for Mint A
        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();
        println!("Taker ATA A: {}\n", taker_ata_a);

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();
        println!("Taker ATA B: {}\n", taker_ata_b);

        MintTo::new(&mut svm, &taker, &mint_b, &taker_ata_b, 1000000000)
            .send()
            .unwrap();

        let maker = payer.pubkey();

        let take_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(maker, false),
                AccountMeta::new_readonly(mint_a, false),
                AccountMeta::new_readonly(mint_b, false),
                AccountMeta::new(escrow, false),
                AccountMeta::new(taker_ata_a, false),
                AccountMeta::new(taker_ata_b, false),
                AccountMeta::new(maker_ata_b, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(token_program, false),
            ],
            data: take_data,
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();

        let transaction = Transaction::new(&[taker], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = svm.send_transaction(transaction).unwrap();

        // Log transaction details
        println!("\n\nTake transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
    }

    #[test]
    pub fn test_cancel_instruction() {
        let (mut svm, payer, taker) = setup();
        let program_id = program_id();

        let (mint_a, _, maker_ata, _, escrow, vault, _, token_program, system_program, bump) =
            helper(&mut svm, &payer, &taker);

        let cancel_data = [
            vec![2u8], // Discriminator for "Cancel" instruction
            bump.to_le_bytes().to_vec(),
        ]
        .concat();

        let cancel_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_a, false),
                AccountMeta::new(escrow, false),
                AccountMeta::new(maker_ata, false),
                AccountMeta::new(vault, false),
                AccountMeta::new_readonly(token_program, false),
                AccountMeta::new_readonly(system_program, false),
            ],
            data: cancel_data,
        };

        let message = Message::new(&[cancel_ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();

        let transaction = Transaction::new(&[payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = svm.send_transaction(transaction).unwrap();

        // Log transaction details
        println!("\n\nCancel transaction successful");
        println!("CUs Consumed: {}", tx.compute_units_consumed);
    }
}

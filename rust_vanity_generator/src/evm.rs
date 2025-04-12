use anyhow::Result;
use bip39::Mnemonic;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use rand::Rng;
use std::io::{self, Write};
use std::process;
use tiny_keccak::{Hasher, Keccak};

pub fn generate_evm_address() -> Result<()> {
    // Default target words
    let default_words = vec![
        "ABCD", "1234", "FADE", "BEAD", "DEAD", "BEEF", "CAFE", "FACE", "BABE",
        "F00D", "C0DE", "FEED", "B00B", "D00D", "BEEF", "CAFE", "DEAD", "FACE", "FEED", "FADE", "BEAD", "BABE"
    ];

    // Get target words from user or use defaults
    println!("Default target words: {}", default_words.join(", "));
    print!("Enter target words (comma separated) or press Enter for defaults: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    
    let target_words = if input.is_empty() {
        default_words.iter().map(|s| s.to_string()).collect::<Vec<String>>()
    } else {
        input.split(',').map(|w| w.trim().to_uppercase()).collect::<Vec<String>>()
    };

    // Get number of threads from user or use default
    let default_threads = num_cpus::get().saturating_sub(4).max(1);
    println!("\nDefault CPU cores: {} (max - 4)", default_threads);
    print!("Enter number of CPU cores to use or press Enter for default: ");
    io::stdout().flush().unwrap();
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    
    let num_threads = if input.is_empty() {
        default_threads
    } else {
        input.parse().unwrap_or_else(|_| {
            println!("Invalid input, using default number of cores");
            default_threads
        })
    };

    println!("\nStarting EVM address generation...");
    println!("Searching for addresses that start AND end with: {}", target_words.join(", "));
    println!("Using {} CPU cores", num_threads);
    println!("Press Ctrl+C to stop");

    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let start_time = std::time::Instant::now();
    let mut handles = vec![];

    for _ in 0..num_threads {
        let counter = counter.clone();
        let target_words = target_words.clone();
        let handle = std::thread::spawn(move || {
            let secp = Secp256k1::new();
            let mut rng = rand::thread_rng();
            let mut last_print = std::time::Instant::now();
            let mut local_counter = 0;

            loop {
                // Generate random entropy for BIP39
                let mut entropy = [0u8; 32];
                rng.fill(&mut entropy);
                
                // Generate mnemonic from entropy
                let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();
                let seed = mnemonic.to_seed("");
                
                // Generate private key from seed
                let private_key = SecretKey::from_slice(&seed[..32]).unwrap();
                
                // Generate public key
                let public_key = private_key.public_key(&secp);
                let public_key_bytes = public_key.serialize_uncompressed();
                
                // Generate EVM address (keccak256 hash of public key, take last 20 bytes)
                let mut keccak = Keccak::v256();
                keccak.update(&public_key_bytes[1..]); // Skip the 0x04 prefix
                let mut hash = [0u8; 32];
                keccak.finalize(&mut hash);
                let address_bytes = &hash[12..]; // Take last 20 bytes
                
                // Convert to hex string
                let address = hex::encode(address_bytes);
                
                // Check if address matches any target word at start and end
                for start_word in &target_words {
                    if address.starts_with(start_word) {
                        for end_word in &target_words {
                            if address.ends_with(end_word) {
                                println!("\nFound match! Address: 0x{}", address);
                                println!("Starts with: {}, Ends with: {}", 
                                    &address[..start_word.len()],
                                    &address[address.len()-end_word.len()..]);
                                
                                println!("Seed Phrase: {}", mnemonic.to_string());
                                
                                // Verify the seed phrase
                                let verified_key = mnemonic.to_seed("");
                                let verified_private_key = SecretKey::from_slice(&verified_key[..32]).unwrap();
                                let verified_public_key = verified_private_key.public_key(&secp);
                                let verified_public_key_bytes = verified_public_key.serialize_uncompressed();
                                
                                let mut keccak = Keccak::v256();
                                keccak.update(&verified_public_key_bytes[1..]);
                                let mut hash = [0u8; 32];
                                keccak.finalize(&mut hash);
                                let verified_address_bytes = &hash[12..];
                                let verified_address = hex::encode(verified_address_bytes);
                                
                                if verified_address == address {
                                    println!("✓ Seed phrase verification successful");
                                } else {
                                    println!("✗ Seed phrase verification failed!");
                                    println!("Expected: 0x{}", address);
                                    println!("Got: 0x{}", verified_address);
                                }
                                
                                process::exit(0);
                            }
                        }
                    }
                }

                local_counter += 1;
                if last_print.elapsed().as_secs() >= 1 {
                    counter.fetch_add(local_counter, std::sync::atomic::Ordering::Relaxed);
                    local_counter = 0;
                    let total = counter.load(std::sync::atomic::Ordering::Relaxed);
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = total as f64 / elapsed;
                    print!("\rGenerated {} addresses ({:.0}/s)", total, rate);
                    io::stdout().flush().unwrap();
                    last_print = std::time::Instant::now();
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
} 
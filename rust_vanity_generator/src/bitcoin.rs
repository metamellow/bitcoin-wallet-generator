use anyhow::Result;
use bip39::Mnemonic;
use bitcoin::Network;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use std::io::{self, Write};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use rand::Rng;

fn get_random_cores(num_cores: usize) -> Vec<usize> {
    let total_cores = num_cpus::get();
    let mut available_cores: Vec<usize> = (0..total_cores).collect();
    let mut selected_cores = Vec::with_capacity(num_cores);
    
    let mut rng = rand::thread_rng();
    
    for _ in 0..num_cores {
        if available_cores.is_empty() {
            break;
        }
        let index = rng.gen_range(0..available_cores.len());
        selected_cores.push(available_cores.remove(index));
    }
    
    selected_cores
}

pub fn generate_btc_address(target_words_arg: Option<String>, threads_arg: Option<usize>) -> Result<()> {
    // Default target words
    let default_words = vec![
        "ABCD", "1234", "FADE", "BEAD", "DEAD", "BEEF", "CAFE", "FACE", "BABE",
        "F00D", "C0DE", "FEED", "B00B", "D00D", "BAE", "BAD", "ACE"
    ];

    // Get target words from user or use defaults
    let target_words = if let Some(words) = target_words_arg {
        words.split(',').map(|w| w.trim().to_uppercase()).collect::<Vec<String>>()
    } else {
        println!("Default target words: {}", default_words.join(", "));
        print!("Enter target words (comma separated) or press Enter for defaults: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        if input.is_empty() {
            default_words.iter().map(|s| s.to_string()).collect::<Vec<String>>()
        } else {
            input.split(',').map(|w| w.trim().to_uppercase()).collect::<Vec<String>>()
        }
    };

    // Get number of threads from user or use default
    let num_threads = if let Some(threads) = threads_arg {
        threads
    } else {
        let default_threads = num_cpus::get().saturating_sub(4).max(1);
        println!("\nDefault CPU cores: {} (max - 4)", default_threads);
        print!("Enter number of CPU cores to use or press Enter for default: ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
        if input.is_empty() {
            default_threads
        } else {
            input.parse().unwrap_or_else(|_| {
                println!("Invalid input, using default number of cores");
                default_threads
            })
        }
    };

    // Get random cores to use
    let selected_cores = get_random_cores(num_threads);
    println!("\nUsing CPU cores: {:?}", selected_cores);

    println!("\nStarting Bitcoin address generation...");
    println!("Searching for addresses that start AND end with: {}", target_words.join(", "));
    println!("Using {} CPU cores", num_threads);
    println!("Press Ctrl+C to stop");

    let counter = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();
    let mut handles = vec![];

    for core in selected_cores {
        let counter = Arc::clone(&counter);
        let target_words = target_words.clone();
        let handle = thread::spawn(move || {
            // Set thread affinity to the selected core
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};
                unsafe {
                    let thread = GetCurrentThread();
                    SetThreadAffinityMask(thread, 1 << core);
                }
            }
            #[cfg(target_os = "linux")]
            {
                use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};
                unsafe {
                    let mut set = std::mem::zeroed::<cpu_set_t>();
                    CPU_ZERO(&mut set);
                    CPU_SET(core, &mut set);
                    sched_setaffinity(0, std::mem::size_of::<cpu_set_t>(), &set);
                }
            }

            let secp = Secp256k1::new();
            let mut rng = rand::thread_rng();
            let mut last_print = Instant::now();
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
                let public_key = bitcoin::PublicKey::new(private_key.public_key(&secp));
                
                // Generate address
                let address = bitcoin::Address::p2pkh(&public_key, Network::Bitcoin);
                let address_str = address.to_string();
                
                // Remove the '1' prefix for matching
                let address_clean = address_str.replace("1", "");
                
                // Check if address matches any target word at start and end
                for start_word in &target_words {
                    if address_clean.starts_with(start_word) {
                        for end_word in &target_words {
                            if address_clean.ends_with(end_word) {
                                println!("\nFound match! Address: {}", address_str);
                                println!("Starts with: {}, Ends with: {}", 
                                    &address_clean[..start_word.len()],
                                    &address_clean[address_clean.len()-end_word.len()..]);
                                
                                println!("Seed Phrase: {}", mnemonic.to_string());
                                
                                // Verify the seed phrase
                                let verified_key = mnemonic.to_seed("");
                                let verified_private_key = SecretKey::from_slice(&verified_key[..32]).unwrap();
                                let verified_public_key = bitcoin::PublicKey::new(verified_private_key.public_key(&secp));
                                let verified_address = bitcoin::Address::p2pkh(&verified_public_key, Network::Bitcoin);
                                
                                if verified_address.to_string() == address_str {
                                    println!("✓ Seed phrase verification successful");
                                } else {
                                    println!("✗ Seed phrase verification failed!");
                                    println!("Expected: {}", address_str);
                                    println!("Got: {}", verified_address.to_string());
                                }
                                
                                process::exit(0);
                            }
                        }
                    }
                }

                local_counter += 1;
                if last_print.elapsed().as_secs() >= 1 {
                    counter.fetch_add(local_counter, Ordering::Relaxed);
                    local_counter = 0;
                    let total = counter.load(Ordering::Relaxed);
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let rate = total as f64 / elapsed;
                    print!("\rGenerated {} addresses ({:.0}/s)", total, rate);
                    io::stdout().flush().unwrap();
                    last_print = Instant::now();
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
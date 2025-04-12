import os
import time
import signal
from eth_account import Account
from mnemonic import Mnemonic
import multiprocessing as mp
from typing import List, Tuple, Optional
import sys

# Enable mnemonic features
Account.enable_unaudited_hdwallet_features()

class VanityAddressGenerator:
    def __init__(self, target_words: List[str], num_cores: int = None):
        self.target_words = target_words
        self.mnemonic = Mnemonic("english")
        self.attempts = mp.Value('i', 0)
        self.start_time = time.time()
        self.stop_event = mp.Event()
        self.result_queue = mp.Queue()
        
        # Set number of cores (default to max - 4)
        if num_cores is None:
            num_cores = max(1, mp.cpu_count() - 4)
        self.num_cores = num_cores

    def generate_seed_and_address(self) -> Tuple[str, str]:
        """Generate a new seed phrase and its corresponding address."""
        # Generate random entropy (16 bytes for 12 words)
        entropy = os.urandom(16)
        
        # Generate mnemonic from entropy
        mnemonic = self.mnemonic.to_mnemonic(entropy)
        
        # Create wallet from mnemonic
        account = Account.from_mnemonic(mnemonic)
        address = account.address
        
        return mnemonic, address

    def check_address(self, address: str) -> bool:
        """Check if the address matches any of the target patterns at both start and end."""
        # Remove 0x prefix
        address = address[2:] if address.startswith('0x') else address
        
        # Check if start matches any target word AND end matches any target word
        # They can be different words from the target list
        for start_word in self.target_words:
            if address.startswith(start_word):
                for end_word in self.target_words:
                    if address.endswith(end_word):
                        return True
        return False

    def worker(self, worker_id: int):
        """Worker process that generates addresses and checks for matches."""
        local_attempts = 0
        last_log_time = time.time()
        
        while not self.stop_event.is_set():
            try:
                local_attempts += 1
                mnemonic, address = self.generate_seed_and_address()
                
                # Log progress every 1000 attempts or every 0.5 seconds
                current_time = time.time()
                if local_attempts % 1000 == 0 or current_time - last_log_time >= 0.5:
                    with self.attempts.get_lock():
                        self.attempts.value += local_attempts
                        local_attempts = 0
                        elapsed = current_time - self.start_time
                        rate = self.attempts.value / elapsed
                        print(f"\rGenerated {self.attempts.value:,} addresses ({rate:,.0f}/s)", end="", flush=True)
                        last_log_time = current_time
                
                if self.check_address(address):
                    print(f"\nFound match! Address: {address}")
                    print(f"Starts with: {address[2:4]}, Ends with: {address[-2:]}")
                    self.result_queue.put((mnemonic, address))
                    self.stop_event.set()
                    return
            except KeyboardInterrupt:
                self.stop_event.set()
                return

    def start(self) -> Optional[Tuple[str, str]]:
        """Start generating addresses with multiple processes."""
        self.attempts.value = 0
        self.start_time = time.time()
        self.stop_event.clear()
        
        processes = []
        for i in range(self.num_cores):
            process = mp.Process(target=self.worker, args=(i,))
            process.start()
            processes.append(process)
        
        # Wait for any process to find a match
        try:
            result = self.result_queue.get(timeout=60)
            self.stop_event.set()
            return result
        except mp.queues.Empty:
            self.stop_event.set()
            return None
        except KeyboardInterrupt:
            print("\nStopping processes...")
            self.stop_event.set()
            return None
        finally:
            for process in processes:
                process.terminate()
                process.join(timeout=1)

def signal_handler(signum, frame):
    print("\nReceived interrupt signal. Stopping...")
    sys.exit(0)

def main():
    # Set up signal handler for Ctrl+C
    signal.signal(signal.SIGINT, signal_handler)
    
    # Get target words from user or use defaults
    default_words = [
        "ABCD", "1234", "FADE", "BEAD", "DEAD", "BEEF", "CAFE", "FACE", "BABE",
        "F00D", "C0DE", "FEED",
        "B00B", "D00D", "BEEF", "CAFE", "DEAD", "FACE", "FEED", "FADE", "BEAD", "BABE"
    ]
    print(f"Default target words: {', '.join(default_words)}")
    user_input = input("Enter target words (comma separated) or press Enter for defaults: ").strip()
    
    if user_input:
        target_words = [word.strip().upper() for word in user_input.split(",")]
    else:
        target_words = default_words
    
    # Get number of cores from user or use default
    default_cores = max(1, mp.cpu_count() - 4)
    print(f"\nDefault CPU cores: {default_cores} (max - 4)")
    user_input = input("Enter number of CPU cores to use or press Enter for default: ").strip()
    
    try:
        num_cores = int(user_input) if user_input else default_cores
        num_cores = max(1, min(num_cores, mp.cpu_count()))  # Ensure valid number of cores
    except ValueError:
        print("Invalid input, using default number of cores")
        num_cores = default_cores
    
    # Create generator
    generator = VanityAddressGenerator(target_words=target_words, num_cores=num_cores)
    
    print("\nStarting address generation...")
    print(f"Searching for addresses that start AND end with: {', '.join(target_words)}")
    print(f"Using {num_cores} CPU cores")
    print("Press Ctrl+C to stop")
    
    try:
        while True:
            result = generator.start()
            if result:
                mnemonic, address = result
                print("\nFound matching address!")
                print(f"Address: {address}")
                print(f"Seed Phrase: {mnemonic}")
                
                # Verify the seed phrase
                account = Account.from_mnemonic(mnemonic)
                if account.address.lower() == address.lower():
                    print("✓ Seed phrase verification successful")
                else:
                    print("✗ Seed phrase verification failed!")
                
                elapsed = time.time() - generator.start_time
                rate = generator.attempts.value / elapsed
                print(f"\nGenerated {generator.attempts.value:,} addresses in {elapsed:.1f}s ({rate:,.0f}/s)")
                
                # Ask if user wants to continue
                if input("\nContinue searching? (y/n): ").lower() != 'y':
                    break
            else:
                print(".", end="", flush=True)
    
    except KeyboardInterrupt:
        print("\nStopped by user")
    finally:
        elapsed = time.time() - generator.start_time
        rate = generator.attempts.value / elapsed
        print(f"\nFinal stats: Generated {generator.attempts.value:,} addresses in {elapsed:.1f}s ({rate:,.0f}/s)")

if __name__ == "__main__":
    main() 
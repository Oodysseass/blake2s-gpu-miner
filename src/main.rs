use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use clap::Parser;
use miner::{miner::mine, block::Block};

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value_t = 8)]
    workers: usize,

    #[arg(short, long, default_value_t = 1)]
    blocks: u64
}

fn main() {
    let args = Args::parse();
    let num_workers = args.workers;
    let target_blocks = args.blocks;

    let total_hashes = Arc::new(AtomicU64::new(0));
    let total_blocks = Arc::new(AtomicU64::new(0));

    let start = std::time::Instant::now();
    let mut handles = vec![];
    for _ in 0..num_workers {
        let total_hashes = Arc::clone(&total_hashes);
        let total_blocks = Arc::clone(&total_blocks);

        let handle = std::thread::spawn(move || {
            loop {
                let block = Block::new(None, Some("benchmark".to_string()), None, vec![]);

                let (_, hashes) = mine(block);
                total_hashes.fetch_add(hashes, Ordering::Relaxed);
                let mined = total_blocks.fetch_add(1, Ordering::Relaxed) + 1;
                if mined > target_blocks {
                    break;
                } 
            }
        });
        handles.push(handle)
    }
    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed().as_secs_f64();
    let hashes = total_hashes.load(Ordering::Relaxed);
    let blocks = total_blocks.load(Ordering::Relaxed);
    println!("Blocks: {}", blocks);
    println!("Total hashes: {}", hashes);
    println!("Elapsed: {:.2}s", elapsed);
    println!("Hashrate: {:.0} hashes/sec", hashes as f64 / elapsed);
    println!("Blocks/min: {:.2}", blocks as f64 / (elapsed / 60.0));
}

use clap::Parser;
use miner::{block::Block, gpu_miner::GpuMiner, miner::mine};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value_t = 8)]
    workers: usize,

    #[arg(short, long, default_value_t = 1)]
    blocks: u64,

    #[arg(short, long, default_value_t = false)]
    gpu: bool,
}

fn new_block() -> Block {
    Block::new(None, Some("benchmark".to_string()), None, vec![])
}

fn print_stats(blocks: u64, hashes: u64, elapsed: f64) {
    println!("Blocks: {}", blocks);
    println!("Total hashes: {}", hashes);
    println!("Elapsed: {:.2}s", elapsed);
    println!("Hashrate: {:.0} hashes/sec", hashes as f64 / elapsed);
    println!("Blocks/min: {:.2}", blocks as f64 / (elapsed / 60.0));
}

fn mine_cpu(target_blocks: u64, num_workers: usize) -> (u64, u64) {
    let total_hashes = Arc::new(AtomicU64::new(0));
    let total_blocks = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];
    for _ in 0..num_workers {
        let total_hashes = Arc::clone(&total_hashes);
        let total_blocks = Arc::clone(&total_blocks);

        handles.push(std::thread::spawn(move || loop {
            let (_, hashes) = mine(new_block());
            total_hashes.fetch_add(hashes, Ordering::Relaxed);
            if total_blocks.fetch_add(1, Ordering::Relaxed) + 1 >= target_blocks {
                break;
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    (
        total_blocks.load(Ordering::Relaxed),
        total_hashes.load(Ordering::Relaxed),
    )
}

async fn mine_gpu(target_blocks: u64) -> (u64, u64) {
    let gpu_miner = GpuMiner::new().await;
    let mut total_hashes: u64 = 0;

    for i in 0..target_blocks {
        let (mined_block, hashes) = gpu_miner.mine(new_block());
        total_hashes += hashes;
        println!(
            "Block {} mined: {}",
            i + 1,
            hex::encode(mined_block.blockid())
        );
    }

    (target_blocks, total_hashes)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let start = std::time::Instant::now();
    let (blocks, hashes) = if args.gpu {
        mine_gpu(args.blocks).await
    } else {
        mine_cpu(args.blocks, args.workers)
    };

    print_stats(blocks, hashes, start.elapsed().as_secs_f64());
}

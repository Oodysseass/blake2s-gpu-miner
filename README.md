# Miner

Blake2s-based blockchain miner with CPU and GPU support.

## Build

```
cargo build --release
```

## Usage

```
cargo run --release -- [OPTIONS]
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-g`, `--gpu` | Use GPU mining | `false` |
| `-w`, `--workers` | Number of CPU threads | `8` |
| `-b`, `--blocks` | Number of blocks to mine | `1` |

### Examples

```bash
# Mine 1 block on CPU with 8 threads
cargo run --release

# Mine 10 blocks on GPU
cargo run --release -- --gpu --blocks 10

# Mine 5 blocks on CPU with 16 threads
cargo run --release -- --workers 16 --blocks 5
```

## Architecture

```
src/
├── main.rs        # CLI, benchmark harness
├── lib.rs         # Module exports
├── block.rs       # Block struct, canonical JSON, blockid
├── blake2s.rs     # Blake2s-256 hash (CPU)
├── miner.rs       # CPU miner (multi-threaded)
└── gpu_miner.rs   # GPU miner (wgpu compute)
shaders/
└── blake2s.wgsl   # Blake2s compute shader (WGSL)
```

### CPU Mining

Spawns `--workers` threads, each independently mining blocks. Each thread picks a random nonce, hashes the block's canonical JSON with Blake2s, and increments the nonce until the hash falls below the target.

### GPU Mining

Uses wgpu to dispatch a compute shader that runs 1M+ Blake2s hashes in parallel per batch. The shader hex-encodes the nonce into the JSON at the correct byte offset, hashes it, and compares against the difficulty target. A result buffer signals the host when a valid nonce is found.

## Tests

```
cargo test
```

Tests verify:
- Blake2s correctness against the reference implementation
- CPU mining produces valid blocks
- GPU mining produces valid blocks (verified by CPU blake2s)
- GPU hash matches CPU hash for the same input
- GPU mining at gradually increasing difficulty levels

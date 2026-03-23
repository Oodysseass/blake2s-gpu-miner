use crate::block::Block;
use bytemuck::{Pod, Zeroable};
use rand::Rng;
use wgpu::util::DeviceExt;

const WORKGROUP_COUNT: u32 = 4096;
const BATCH_SIZE: u32 = WORKGROUP_COUNT * 256;

pub struct GpuMiner {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    input_len: u32,
    nonce_byte_offset: u32,
    batch_offset: u32,
}

impl GpuMiner {
    pub async fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Adapter not found");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Device and queue could not be instantiated");

        let shader_str = include_str!("../shaders/blake2s.wgsl");
        let shader_source = wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(shader_str));
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: shader_source,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let miner = GpuMiner {
            device,
            queue,
            pipeline,
        };
        miner
    }

    pub fn mine(&self, mut block: Block) -> (Block, u64) {
        let block_json = block.canonical_json();

        let prefix = "\"nonce\":\"";
        let prefix_index = block_json.find(prefix).unwrap();
        let nonce_byte_offset = u32::try_from(prefix_index + prefix.len()).unwrap();

        let difficulty: Vec<u32> = hex::decode(&block.T)
            .unwrap()
            .chunks(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        let input = block_json.as_bytes();
        let mut params = Params {
            input_len: u32::try_from(block_json.len()).unwrap(),
            nonce_byte_offset,
            batch_offset: 0_u32,
        };

        let input_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: input,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let difficulty_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&difficulty),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let params_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let mut nonce = [0_u8; 32];
        rand::thread_rng().fill(&mut nonce);
        let mut nonce_words: Vec<u32> = nonce
            .chunks(4)
            .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        nonce_words.push(0);

        let result_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&nonce_words),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            });
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (9 * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: difficulty_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: result_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut total_hashes: u64 = 0;
        loop {
            total_hashes += BATCH_SIZE as u64;
            let mut command_encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(WORKGROUP_COUNT, 1, 1);
            drop(compute_pass);

            command_encoder.copy_buffer_to_buffer(
                &result_buffer,
                0,
                &staging_buffer,
                0,
                (9 * std::mem::size_of::<u32>()) as u64,
            );

            let command_buffer = command_encoder.finish();
            self.queue.submit([command_buffer]);

            staging_buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, |result| {
                    if result.is_err() {
                        panic!("Staging buffer mapping failed");
                    }
                });
            let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
            let view = staging_buffer.slice(..).get_mapped_range();
            let result: &[u32] = bytemuck::cast_slice(&view);

            if result[8] == 1 {
                let valid_nonce = hex::encode(
                    result[0..8]
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .collect::<Vec<u8>>(),
                );
                block.nonce = valid_nonce;
                drop(view);
                staging_buffer.unmap();
                return (block, total_hashes);
            }

            drop(view);
            staging_buffer.unmap();

            params.batch_offset = params.batch_offset.wrapping_add(BATCH_SIZE);
            if params.batch_offset < BATCH_SIZE {
                let mut nonce = [0_u8; 32];
                rand::thread_rng().fill(&mut nonce);
                let mut nonce_words: Vec<u32> = nonce
                    .chunks(4)
                    .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
                    .collect();
                nonce_words.push(0);
                self.queue
                    .write_buffer(&result_buffer, 0, bytemuck::cast_slice(&nonce_words));
                params.batch_offset = 0;
            }
            self.queue
                .write_buffer(&params_buffer, 0, &bytemuck::bytes_of(&params));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blake2s::blake2s;

    async fn verify_gpu_mine(target: &str) {
        let mut block = Block::new(None, Some("test".to_string()), None, vec![]);
        block.T = target.to_string();

        let gpu_miner = GpuMiner::new().await;
        let start = std::time::Instant::now();
        let (mined_block, hashes) = gpu_miner.mine(block);
        let elapsed = start.elapsed().as_secs_f64();

        let block_json = mined_block.canonical_json();
        let cpu_hash = blake2s(block_json.as_bytes());
        let target_bytes: [u8; 32] = hex::decode(&mined_block.T).unwrap().try_into().unwrap();

        println!("Target:           {}", mined_block.T);
        println!("Nonce:            {}", mined_block.nonce);
        println!("Block hash (CPU): {}", hex::encode(cpu_hash));
        println!("Hashes: {}  Time: {:.3}s  Hashrate: {:.0} H/s", hashes, elapsed, hashes as f64 / elapsed);

        assert!(
            cpu_hash < target_bytes,
            "GPU-mined block hash {} is not below target {}",
            hex::encode(cpu_hash),
            mined_block.T
        );
    }

    #[tokio::test]
    async fn test_gpu_hash_matches_cpu() {
        let block = Block::new(None, Some("test".to_string()), None, vec![]);
        let block_json = block.canonical_json();

        let cpu_hash = blake2s(block_json.as_bytes());

        let gpu_miner = GpuMiner::new().await;
        let mut gpu_block = block;
        gpu_block.T = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
        let (mined_block, _) = gpu_miner.mine(gpu_block);

        let mined_json = mined_block.canonical_json();
        let gpu_result_hash = blake2s(mined_json.as_bytes());

        println!("CPU hash of original: {}", hex::encode(cpu_hash));
        println!("CPU hash of GPU-mined: {}", hex::encode(gpu_result_hash));
        println!("GPU nonce: {}", mined_block.nonce);

        let target: [u8; 32] = hex::decode(&mined_block.T).unwrap().try_into().unwrap();
        assert!(gpu_result_hash < target);
    }

    #[tokio::test]
    async fn test_gpu_mine_gradual_difficulty() {
        let targets = [
            "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "00ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "0000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        ];

        let gpu_miner = GpuMiner::new().await;

        for target in targets {
            let mut block = Block::new(None, Some("test".to_string()), None, vec![]);
            block.T = target.to_string();

            let start = std::time::Instant::now();
            let (mined_block, hashes) = gpu_miner.mine(block);
            let elapsed = start.elapsed().as_secs_f64();

            let block_json = mined_block.canonical_json();
            let cpu_hash = blake2s(block_json.as_bytes());
            let target_bytes: [u8; 32] = hex::decode(&mined_block.T).unwrap().try_into().unwrap();

            println!(
                "Target: {}  Hash: {}  Hashes: {}  Time: {:.3}s  Rate: {:.0} H/s",
                &target[..8],
                &hex::encode(cpu_hash)[..8],
                hashes,
                elapsed,
                hashes as f64 / elapsed
            );

            assert!(
                cpu_hash < target_bytes,
                "GPU-mined block hash {} is not below target {}",
                hex::encode(cpu_hash),
                target
            );
        }
    }
}

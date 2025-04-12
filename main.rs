use ocl::{core, flags};
use ocl::enums::{ArgVal, DeviceInfo};
use ocl::builders::ContextProperties;
use rayon::prelude::*;
use bitcoin_wallet::mnemonic::Mnemonic;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::fs::OpenOptions;
use std::io::Write;
use serde::{Deserialize, Serialize};
use serde_json;
use log::{info, error};
use simple_logger;

// Constants
const KNOWN_WORDS: [&str; 9] = [
    "basic", "town", "town", "town",
    "abandon", "basic", "basic", "basic", "basic",
];
const BATCH_SIZE: u64 = 1000;
const TARGET: &str = "0x57266b1ca310c964a111b4c3a19b1448ea725356";

// Global Atomic Counter
static CURRENT_OFFSET: AtomicU64 = AtomicU64::new(0);

// Mutex for thread-safe logging
lazy_static::lazy_static! {
    static ref SOLUTION_LOG: Mutex<()> = Mutex::new(());
}

// Data structure for logging solutions
#[derive(Serialize, Deserialize)]
struct Solution {
    offset: u64,
    mnemonic: String,
}

// Get the next workload
fn get_next_work() -> (u64, u64) {
    let offset = CURRENT_OFFSET.fetch_add(BATCH_SIZE, Ordering::SeqCst);
    (offset, BATCH_SIZE)
}

// Log solutions to a file
fn log_solution(offset: u64, mnemonic: String) {
    let _lock = SOLUTION_LOG.lock().unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("solutions.log")
        .unwrap();
    let solution = Solution { offset, mnemonic };
    let json = serde_json::to_string(&solution).unwrap();
    writeln!(file, "{}", json).unwrap();
}

// GPU computation kernel logic
fn mnemonic_gpu(platform_id: ocl::PlatformId, device_id: ocl::DeviceId, src: String) {
    let context_properties = ContextProperties::new().platform(platform_id);
    let context = core::create_context(Some(&context_properties), &[device_id], None, None).unwrap();
    let program = core::create_program_with_source(&context, &[src]).unwrap();
    core::build_program(&program, Some(&[device_id]), &CString::new("").unwrap(), None, None)
        .unwrap();
    let queue = core::create_command_queue(&context, &device_id, None).unwrap();

    loop {
        let (offset, batch_size) = get_next_work();

        // Prepare arguments for kernel execution
        let mnemonic_hi = (offset >> 32) as u64;
        let mnemonic_lo = (offset & 0xFFFFFFFF) as u64;

        let mut target_mnemonic = vec![0u8; 120];
        let mut mnemonic_found = vec![0u8; 1];

        let target_mnemonic_buf = unsafe {
            core::create_buffer(
                &context,
                flags::MEM_WRITE_ONLY | flags::MEM_COPY_HOST_PTR,
                120,
                Some(&target_mnemonic),
            )
            .unwrap()
        };

        let mnemonic_found_buf = unsafe {
            core::create_buffer(
                &context,
                flags::MEM_WRITE_ONLY | flags::MEM_COPY_HOST_PTR,
                1,
                Some(&mnemonic_found),
            )
            .unwrap()
        };

        let kernel = core::create_kernel(&program, "mnemonic_kernel").unwrap();

        core::set_kernel_arg(&kernel, 0, ArgVal::scalar(&mnemonic_hi)).unwrap();
        core::set_kernel_arg(&kernel, 1, ArgVal::scalar(&mnemonic_lo)).unwrap();
        core::set_kernel_arg(&kernel, 2, ArgVal::mem(&target_mnemonic_buf)).unwrap();
        core::set_kernel_arg(&kernel, 3, ArgVal::mem(&mnemonic_found_buf)).unwrap();

        unsafe {
            core::enqueue_kernel(
                &queue,
                &kernel,
                1,
                None,
                &[batch_size as usize, 1, 1],
                None,
                None::<core::Event>,
                None::<&mut core::Event>,
            )
            .unwrap();
        }

        unsafe {
            core::enqueue_read_buffer(
                &queue,
                &target_mnemonic_buf,
                true,
                0,
                &mut target_mnemonic,
                None::<core::Event>,
                None::<&mut core::Event>,
            )
            .unwrap();
        }

        unsafe {
            core::enqueue_read_buffer(
                &queue,
                &mnemonic_found_buf,
                true,
                0,
                &mut mnemonic_found,
                None::<core::Event>,
                None::<&mut core::Event>,
            )
            .unwrap();
        }

        if mnemonic_found[0] == 0x01 {
            let mnemonic = String::from_utf8(target_mnemonic)
                .unwrap()
                .trim_matches(char::from(0))
                .to_string();
            log_solution(offset, mnemonic);
            break;
        }
    }
}

fn main() {
    simple_logger::init().unwrap();
    info!("Starting BIP39 brute-force script...");

    let platform_id = core::default_platform().unwrap();
    let device_ids = core::get_device_ids(&platform_id, Some(flags::DEVICE_TYPE_GPU), None).unwrap();

    let kernel_source = std::fs::read_to_string("cl/mnemonic.cl").expect("Failed to load kernel file");

    device_ids.into_par_iter().for_each(|device_id| {
        info!("Starting work on device: {:?}", device_id);
        mnemonic_gpu(platform_id, device_id, kernel_source.clone());
    });
}
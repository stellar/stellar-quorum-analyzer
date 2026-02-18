use crate::allocator::{get_memory_usage, set_memory_limit};
use std::process::Command;

// This test must run in an isolated process because it overrides the global
// allocator's memory limit. It is invoked by `test_allocator_limit_precision`
// below as a subprocess.
#[ignore]
#[test]
fn test_allocator_limit_precision_ps() {
    let args: Vec<String> = std::env::args().collect();
    let memory_limit: usize = if args.len() > 4 {
        args[4].parse().unwrap()
    } else {
        10_000_000
    };

    set_memory_limit(memory_limit);
    let baseline = get_memory_usage();

    // Allocate chunks that fit within the limit minus baseline
    let alloc_size = 100_000;
    let budget = memory_limit.saturating_sub(baseline + alloc_size);
    let max_allocs = budget / alloc_size;
    let mut vecs: Vec<Vec<u8>> = Vec::new();
    for _ in 0..max_allocs {
        vecs.push(vec![0u8; alloc_size]);
    }

    let used = get_memory_usage() - baseline;
    // With the old bug (off-by-one using fetch_add's return value as the new
    // total), the allocator would allow one extra allocation beyond the limit.
    // After the fix, old_size + layout.size() is checked correctly.
    assert!(
        used <= memory_limit,
        "Memory usage {used} should not exceed limit {memory_limit}"
    );

    // Verify the counter is tracking accurately
    let expected_min = max_allocs * alloc_size;
    assert!(
        used >= expected_min,
        "Memory usage {used} should be at least {expected_min} ({max_allocs}x{alloc_size})"
    );
}

#[test]
fn test_allocator_limit_precision() -> Result<(), Box<dyn std::error::Error>> {
    // Run the subprocess test with a 10MB limit — should succeed
    let output = Command::new("cargo")
        .args([
            "test",
            "--package",
            "stellar-quorum-analyzer",
            "--lib",
            "--",
            "test_allocator_limit_precision_ps",
            "--include-ignored",
            "--nocapture",
            "10000000",
        ])
        .output()?;
    assert_eq!(
        output.status.code(),
        Some(0),
        "Subprocess should succeed within memory limit. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

pub(crate) fn measure() -> xtask::Result<()> {
    let mut samples = [0_u128; 1024];
    for sample in &mut samples {
        let start = std::time::Instant::now();
        std::hint::black_box(xtask::audit::mca_throughput(std::hint::black_box(
            "Block RThroughput: 1.5",
        ))?);
        *sample = start.elapsed().as_nanos();
    }
    samples.sort_unstable();
    println!(
        "mca_report: P50={}ns P90={}ns P99={}ns P99.9={}ns",
        samples[512], samples[921], samples[1013], samples[1022]
    );
    Ok(())
}

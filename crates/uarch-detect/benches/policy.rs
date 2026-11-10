pub(super) fn run() {
    crate::sample("classify", || {
        uarch_detect::classify(
            &uarch_detect::HardwareCpuid,
            std::hint::black_box(uarch_detect::DetectPolicy::default()),
        )
    });
}

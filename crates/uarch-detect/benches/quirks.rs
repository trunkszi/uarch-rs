pub(super) fn run() {
    crate::sample("pdep_cost", || {
        uarch_detect::quirks::pdep_cost(&uarch_detect::HardwareCpuid)
    });
}

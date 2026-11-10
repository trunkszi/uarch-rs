pub(super) fn run() {
    crate::sample("neon_hwcap", uarch_detect::hwcap::neon_supported);
}

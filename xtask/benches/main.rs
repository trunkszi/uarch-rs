mod audit;
fn main() -> xtask::Result<()> {
    audit::measure()
}

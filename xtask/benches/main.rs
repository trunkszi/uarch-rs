#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod audit;
fn main() -> xtask::Result<()> {
    audit::measure()
}

#[cfg(feature = "detect")]
use uarch::arch::{Arch, Scalar, Tier};
#[cfg(feature = "detect")]
use uarch::detect::{DetectPolicy, detect};
#[cfg(feature = "detect")]
use uarch::tiers::TierId;

#[cfg(feature = "detect")]
#[test]
fn detect_reports_host_tier() {
    match detect(DetectPolicy::default()) {
        Ok(tier) => match tier {
            Tier::Scalar(token) => {
                assert_eq!(Scalar::TIER, TierId::Scalar);
                assert_eq!(token.bit_gather(0b1011, 0b0101), 1);
            }
            Tier::X86V3(token) => {
                let _ = token.bit_gather(1, 1);
            }
            Tier::X86V3SlowPdep(token) => {
                let _ = token.bit_gather(1, 1);
            }
            Tier::X86V4(token) => {
                let _ = token.bit_gather(1, 1);
            }
            Tier::Aarch64Neon(token) => {
                let _ = token.bit_gather(1, 1);
            }
        },
        Err(err) => {
            let text = format!("{err}");
            assert!(!text.is_empty());
        }
    }
}

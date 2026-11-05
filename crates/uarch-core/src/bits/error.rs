/// A rejected bit-field value or contradictory composition.
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitError {
    #[error("[bits.value] value exceeds field width")]
    ValueOverflow,
    #[error("[bits.conflict] overlapping bits disagree")]
    Conflict,
}

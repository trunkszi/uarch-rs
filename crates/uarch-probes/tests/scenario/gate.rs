use uarch_probes::{AbiRule, GATES};

#[test]
fn gate_registry_is_auditable() {
    for (index, gate) in GATES.iter().enumerate() {
        assert!(!gate.symbol.is_empty());
        assert!(
            !GATES[..index]
                .iter()
                .any(|other| other.symbol == gate.symbol)
        );
        assert!(
            !gate
                .require
                .iter()
                .any(|instruction| gate.deny.contains(instruction))
        );
        assert!(gate.max_instructions.is_none_or(|limit| limit > 0));
        assert!(
            gate.mca
                .is_none_or(|bound| bound.max_block_reciprocal_throughput.is_finite()
                    && bound.max_block_reciprocal_throughput > 0.0)
        );
    }
    assert!(GATES.iter().any(|gate| gate.abi == AbiRule::Registers));
    assert!(GATES.iter().any(|gate| gate.abi == AbiRule::MemoryControl));
}

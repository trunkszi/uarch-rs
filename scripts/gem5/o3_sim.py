#!/usr/bin/env python3
"""Parametric O3 SE profiles; inspired-by labels do not certify vendor timing."""
import argparse
import json
import sys
from pathlib import Path

PROFILES = {
    "o3-w6-rob320": dict(width=6, rob=320, iq=160, lq=72, sq=64,
                         l1d="32KiB", l1d_assoc=8, l2="1MiB", isa="x86_64", inspired_by="znver4"),
    "o3-w8-rob448": dict(width=8, rob=448, iq=192, lq=96, sq=72,
                         l1d="48KiB", l1d_assoc=12, l2="1MiB", isa="x86_64", inspired_by="znver5"),
    "o3-w6-rob512": dict(width=6, rob=512, iq=196, lq=128, sq=72,
                         l1d="48KiB", l1d_assoc=12, l2="2MiB", isa="x86_64", inspired_by="sapphirerapids"),
    "o3-a64-w6-rob320": dict(width=6, rob=320, iq=160, lq=84, sq=72,
                             l1d="64KiB", l1d_assoc=8, l2="1MiB", isa="aarch64", inspired_by="neoverse-v2"),
}
ALIASES = {
    "zen4-o3": "o3-w6-rob320", "znver4": "o3-w6-rob320",
    "zen5-o3": "o3-w8-rob448", "znver5": "o3-w8-rob448",
    "sapphirerapids-o3": "o3-w6-rob512", "sapphirerapids": "o3-w6-rob512",
    "neoverse-v2-o3": "o3-a64-w6-rob320", "neoverse-v2": "o3-a64-w6-rob320",
}


def profile_data(name):
    """Return a complete model description without importing the simulator."""
    canonical = ALIASES.get(name, name)
    if canonical not in PROFILES:
        raise ValueError("unknown O3 profile: " + name)
    return dict(PROFILES[canonical], canonical=canonical, clock="2GHz", memory="512MiB",
                slot_width=PROFILES[canonical]["width"], attribution_mode="o3-rename-slot",
                predictor=dict(type="TournamentBP", localPredictorSize=2048, localCtrBits=2,
                               localHistoryTableSize=2048, globalPredictorSize=8192,
                               globalCtrBits=2, choicePredictorSize=8192, choiceCtrBits=2),
                functional_units=dict(int_alu_latency=1, int_mult_latency=3, int_div_latency=20,
                                      int_div_pipelined=False, memory_ports=2, simd_latency=1),
                l1i="32KiB", l1i_assoc=8, l1_tag_latency=2, l1_data_latency=2,
                l1_response_latency=1, l2_tag_latency=2, l2_data_latency=8,
                l2_response_latency=2, memory_latency="50ns", memory_bandwidth="16GiB/s",
                l1_mshrs=16, l2_mshrs=32, targets_per_mshr=16,
                physical_integer_regs=768, physical_float_regs=768,
                physical_vector_regs=768, physical_cc_regs=768)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", default="o3-w6-rob320", help="Canonical O3 profile or inspired-by alias")
    parser.add_argument("--dump-profile-json", action="store_true", help="Print model parameters without gem5")
    parser.add_argument("--binary", help="Baseline-ISA guest executable")
    parser.add_argument("--probe", help="Exact exported probe symbol")
    parser.add_argument("--iterations", help="Positive fixed-width ten-digit decimal count")
    args = parser.parse_args()
    try:
        p = profile_data(args.profile)
    except ValueError as error:
        parser.error(str(error))
    if args.dump_profile_json:
        print(json.dumps(p, sort_keys=True))
        return
    if not args.binary or not args.probe or not args.iterations:
        parser.error("--binary, --probe and --iterations are required")
    if len(args.iterations) != 10 or not args.iterations.isascii() or not args.iterations.isdigit() or int(args.iterations) <= 0:
        parser.error("iterations must be positive fixed-width ten-digit decimal")
    binary = Path(args.binary).resolve(strict=True)
    with binary.open("rb") as elf:
        header = elf.read(20)
    expected_machine = 62 if p["isa"] == "x86_64" else 183
    if len(header) < 20 or header[:6] != b"\x7fELF\x02\x01" or int.from_bytes(header[18:20], "little") != expected_machine:
        parser.error("profile and guest ELF ISA differ")

    import m5
    from m5 import objects as obj

    cpu_type = getattr(obj, "X86O3CPU" if p["isa"] == "x86_64" else "ArmO3CPU")
    system = obj.System()
    system.clk_domain = obj.SrcClockDomain(clock=p["clock"], voltage_domain=obj.VoltageDomain())
    system.mem_mode = "timing"
    system.mem_ranges = [obj.AddrRange(p["memory"])]
    system.cache_line_size = 64
    cpu = cpu_type()
    system.cpu = cpu
    for key in ("fetchWidth", "decodeWidth", "renameWidth", "dispatchWidth", "issueWidth", "wbWidth", "commitWidth", "squashWidth"):
        setattr(cpu, key, p["width"])
    for key, value in {"numROBEntries": p["rob"],
                       "LQEntries": p["lq"], "SQEntries": p["sq"],
                       "numPhysIntRegs": p["physical_integer_regs"], "numPhysFloatRegs": p["physical_float_regs"],
                       "numPhysVecRegs": p["physical_vector_regs"], "numPhysCCRegs": p["physical_cc_regs"]}.items():
        setattr(cpu, key, value)
    predictor = dict(p["predictor"])
    predictor.pop("type")
    bp = obj.TournamentBP(**predictor)
    if hasattr(obj, "BranchPredictor") and "conditionalBranchPred" in obj.BranchPredictor._params:
        cpu.branchPred = obj.BranchPredictor(conditionalBranchPred=bp)
    else:
        cpu.branchPred = bp
    fu = p["functional_units"]
    pool = obj.FUPool(FUList=[
        obj.FUDesc(opList=[obj.OpDesc(opClass="IntAlu", opLat=fu["int_alu_latency"])], count=p["width"]),
        obj.FUDesc(opList=[obj.OpDesc(opClass="IntMult", opLat=fu["int_mult_latency"]),
                          obj.OpDesc(opClass="IntDiv", opLat=fu["int_div_latency"], pipelined=False)], count=2),
        obj.FUDesc(opList=[obj.OpDesc(opClass=kind) for kind in ("MemRead", "MemWrite", "FloatMemRead", "FloatMemWrite")], count=fu["memory_ports"]),
        obj.FUDesc(opList=[obj.OpDesc(opClass=kind, opLat=fu["simd_latency"]) for kind in
                          ("FloatAdd", "FloatCmp", "FloatCvt", "FloatMult", "FloatDiv", "FloatMisc", "FloatSqrt",
                           "SimdAdd", "SimdAlu", "SimdCmp", "SimdMisc", "SimdMult", "SimdShift", "SimdReduceAdd",
                           "SimdFloatAdd", "SimdFloatCmp", "SimdFloatReduceAdd",
                           "SimdCvt", "SimdFloatAlu", "SimdFloatCvt", "SimdFloatDiv", "SimdFloatMisc",
                           "SimdFloatMult", "SimdFloatMultAcc", "SimdFloatSqrt")], count=4),
    ])

    if hasattr(obj, "IQUnit"):
        cpu.instQueues = [obj.IQUnit(numEntries=p["iq"], fuPool=pool)]
    else:
        cpu.numIQEntries = p["iq"]
        cpu.fuPool = pool

    def cache(size, assoc, level):
        return obj.Cache(size=size, assoc=assoc, tag_latency=p[level + "_tag_latency"],
                         data_latency=p[level + "_data_latency"], response_latency=p[level + "_response_latency"],
                         mshrs=p[level + "_mshrs"], tgts_per_mshr=p["targets_per_mshr"])

    cpu.icache = cache(p["l1i"], p["l1i_assoc"], "l1")
    cpu.dcache = cache(p["l1d"], p["l1d_assoc"], "l1")
    system.l2bus = obj.L2XBar()
    system.l2 = cache(p["l2"], 16, "l2")
    system.membus = obj.SystemXBar()
    cpu.icache_port = cpu.icache.cpu_side
    cpu.dcache_port = cpu.dcache.cpu_side
    cpu.icache.mem_side = system.l2bus.cpu_side_ports
    cpu.dcache.mem_side = system.l2bus.cpu_side_ports
    system.l2bus.mem_side_ports = system.l2.cpu_side
    system.l2.mem_side = system.membus.cpu_side_ports
    system.memory = obj.SimpleMemory(range=system.mem_ranges[0], latency=p["memory_latency"], bandwidth=p["memory_bandwidth"])
    system.memory.port = system.membus.mem_side_ports
    system.system_port = system.membus.cpu_side_ports
    cpu.createInterruptController()
    if p["isa"] == "x86_64":
        cpu.interrupts[0].pio = system.membus.mem_side_ports
        cpu.interrupts[0].int_requestor = system.membus.cpu_side_ports
        cpu.interrupts[0].int_responder = system.membus.mem_side_ports
    system.workload = obj.SEWorkload.init_compatible(str(binary))
    process = obj.Process(cmd=[str(binary), "__gem5-probe", "--probe", args.probe, "--iterations", args.iterations])
    if hasattr(process, "release"):
        process.release = "6.8.0"
    process.env = ["GLIBC_TUNABLES=glibc.pthread.rseq=0"]
    cpu.workload = process
    cpu.createThreads()
    root = obj.Root(full_system=False, system=system)
    m5.disableAllListeners()
    m5.instantiate()
    event = m5.simulate(100_000_000_000)
    if event.getCause() != "exiting with last active thread context" or event.getCode() != 0:
        sys.stderr.write("guest did not complete: " + event.getCause() + "\n")
        sys.exit(2)
    # gem5 emits the final statistics block at normal shutdown.
    _ = root


if __name__ in ("__main__", "__m5_main__"):
    main()

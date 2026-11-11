use super::*;
#[test]
fn simple_roundtrip_and_all_workloads_validate() {
    let mut paths: Vec<_> =
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/benchmarks/programs"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "lep"))
            .collect();
    paths.sort();
    for path in paths {
        let source = std::fs::read_to_string(path).unwrap();
        let bytes = compile_exb(&source, "test.lep").unwrap();
        let program = load_exb(&bytes).unwrap();
        assert_eq!(program.source(), source);
        assert_eq!(bytes, compile_exb(&source, "test.lep").unwrap());
    }
}

fn fixture(source: &str) -> CompiledProgram {
    load_exb(&compile_exb(source, "test.lep").unwrap()).unwrap()
}
fn forged(program: &CompiledProgram) -> Vec<u8> {
    let mut w = Writer {
        bytes: vec![],
        items: MAX_ITEMS,
    };
    w.string(&program.source_name).unwrap();
    w.string(&program.source).unwrap();
    w.span(program.span).unwrap();
    w.chunk(&program.code, 0).unwrap();
    let mut bytes = Vec::new();
    bytes.extend(MAGIC);
    bytes.extend(EXB_VERSION.to_le_bytes());
    bytes.extend(FLAGS.to_le_bytes());
    bytes.extend((w.bytes.len() as u32).to_le_bytes());
    bytes.extend(checksum(&w.bytes).to_le_bytes());
    bytes.extend(w.bytes);
    bytes
}
#[test]
fn rejects_truncation_version_flags_checksum_and_trailing_data() {
    let bytes = compile_exb("escreva(1)", "test.lep").unwrap();
    for end in 0..bytes.len() {
        assert!(load_exb(&bytes[..end]).is_err(), "truncation {end}");
    }
    for offset in [0, 4, 6, 8, 12, HEADER + 1] {
        let mut broken = bytes.clone();
        broken[offset] ^= 0x80;
        assert!(load_exb(&broken).is_err());
    }
    let mut broken = bytes;
    broken.push(0);
    assert!(load_exb(&broken).is_err());
}
#[test]
fn rejects_valid_checksum_programs_with_unsafe_stack_and_scope_shapes() {
    let cases = vec![
        vec![Op::Drop],
        vec![Op::SetLast],
        vec![Op::Finish],
        vec![Op::Constant(u32::MAX)],
        vec![Op::LoadLocal(0)],
        vec![Op::Call {
            argc: u32::MAX,
            name: 0,
        }],
        vec![Op::Jump(u32::MAX)],
        vec![Op::LeaveScope],
        vec![Op::RestoreScope],
        vec![Op::ParentScope],
        vec![Op::EndTry],
        vec![Op::LoopBody],
        vec![Op::LoopEnd],
        vec![Op::LoopNext(0)],
        vec![Op::ReadDestination],
        vec![Op::WriteDestination],
        vec![Op::Jump(0)],
        vec![
            Op::Bool(false),
            Op::BranchFalse(4),
            Op::Nada,
            Op::Jump(4),
            Op::Nada,
            Op::Finish,
        ],
        vec![
            Op::Nada,
            Op::Try(5),
            Op::Drop,
            Op::Nada,
            Op::EndTry,
            Op::Finish,
        ],
    ];
    for ops in cases {
        let mut p = fixture("escreva(1)");
        let c = Rc::get_mut(&mut p.code).unwrap();
        c.instructions = ops
            .into_iter()
            .map(|op| Instruction { op, span: 0 })
            .collect();
        assert!(load_exb(&forged(&p)).is_err());
    }
}
#[test]
fn malformed_metadata_and_decimal_flags_are_rejected() {
    let mut p = fixture("repita 1 vezes { 1 }");
    Rc::get_mut(&mut p.code).unwrap().loops[0].exit = u32::MAX;
    assert!(load_exb(&forged(&p)).is_err());
    let mut p = fixture("escreva(1)");
    Rc::get_mut(&mut p.code).unwrap().spans[0].line = u32::MAX;
    assert!(load_exb(&forged(&p)).is_err());
    let mut p = fixture("escreva(1)");
    Rc::get_mut(&mut p.code).unwrap().spans[0].end = u32::MAX as usize;
    assert!(load_exb(&forged(&p)).is_err());
    for flags in [1u32, 0x001d0000, 0x01000000, u32::MAX] {
        let mut bytes = [0; 16];
        bytes[..4].copy_from_slice(&flags.to_le_bytes());
        assert!(Numero::from_exb_bytes(bytes).is_none());
    }
}
#[test]
fn exact_decimal_representation_roundtrips() {
    for raw in [
        "0.0000",
        "1.2300",
        "79228162514264337593543950335",
        "0.0000000000000000000000000001",
        "-0.0",
        "-123.4500",
    ] {
        let n = parse_numero(raw).unwrap();
        let bytes = n.exb_bytes();
        assert_eq!(Numero::from_exb_bytes(bytes).unwrap().exb_bytes(), bytes);
    }
}
#[test]
fn checked_control_flow_is_cancellable_and_source_is_not_reparsed() {
    let mut p = fixture("escreva(1)");
    p.source = Rc::from("!!!!!!!!!!");
    let bytes = forged(&p);
    let output = crate::runtime::run_exb_with_options(
        &bytes,
        "test.exb",
        "",
        crate::runtime::ExecutionOptions {
            backend: ExecutionBackend::Bytecode,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(output.stdout, "1\n");
    let mut p = fixture("1");
    Rc::get_mut(&mut p.code).unwrap().instructions = vec![
        Instruction {
            op: Op::Checkpoint,
            span: 0,
        },
        Instruction {
            op: Op::Jump(0),
            span: 0,
        },
    ];
    let bytes = forged(&p);
    let result = crate::runtime::run_exb_with_options(
        &bytes,
        "test.exb",
        "",
        crate::runtime::ExecutionOptions {
            backend: ExecutionBackend::Bytecode,
            time_limit: Some(Duration::from_millis(1)),
            ..Default::default()
        },
    );
    assert!(result.unwrap_err().message.contains("tempo esgotado"));
}
#[test]
fn arbitrary_mutations_do_not_panic_the_loader() {
    let original = compile_exb("f = funcao(x) { x + 1 }\nescreva(f(2))", "test.lep").unwrap();
    for i in 0..512 {
        let mut bytes = original.clone();
        let pos = HEADER + (i * 7919) % (bytes.len() - HEADER);
        bytes[pos] ^= (i as u8).wrapping_mul(17) | 1;
        let hash = checksum(&bytes[HEADER..]);
        bytes[12..20].copy_from_slice(&hash.to_le_bytes());
        assert!(
            std::panic::catch_unwind(|| load_exb(&bytes)).is_ok(),
            "mutation {i}, byte {pos}"
        );
    }
}

#[test]
fn every_wire_opcode_roundtrips_with_full_operands() {
    let cases = [
        Op::Nada,
        Op::Drop,
        Op::SetLast,
        Op::Pair,
        Op::Index,
        Op::Index2,
        Op::Return,
        Op::Finish,
        Op::Checkpoint,
        Op::EnterScope,
        Op::LeaveScope,
        Op::ParentScope,
        Op::RestoreScope,
        Op::CheckBool,
        Op::CheckInt,
        Op::EndTry,
        Op::LoopBody,
        Op::LoopEnd,
        Op::Break,
        Op::Continue,
        Op::ReceiverIndex,
        Op::ReceiverIndex2,
        Op::MatrixDestination,
        Op::ReadDestination,
        Op::WriteDestination,
        Op::Constant(17),
        Op::BadNumber(17),
        Op::Load(17),
        Op::Store(17),
        Op::LoadLocal(17),
        Op::StoreLocal(17),
        Op::List(17),
        Op::Field(17),
        Op::MapField(17),
        Op::Closure(17),
        Op::Jump(17),
        Op::BranchFalse(17),
        Op::Try(17),
        Op::Import(17),
        Op::LoopEnter(17),
        Op::LoopNext(17),
        Op::NameDestination(17),
        Op::ReceiverName(17),
        Op::ReceiverMap(17),
        Op::MapDestination(17),
        Op::Bool(false),
        Op::Bool(true),
        Op::Slice(false),
        Op::Slice(true),
        Op::ReceiverReadOnly(false),
        Op::ReceiverReadOnly(true),
        Op::IndexDestination(false),
        Op::IndexDestination(true),
        Op::Unary(UnaryOp::Neg),
        Op::Unary(UnaryOp::Not),
        Op::Binary(BinaryOp::Add),
        Op::Binary(BinaryOp::Sub),
        Op::Binary(BinaryOp::Mul),
        Op::Binary(BinaryOp::Div),
        Op::Binary(BinaryOp::Rem),
        Op::Binary(BinaryOp::Eq),
        Op::Binary(BinaryOp::Ne),
        Op::Binary(BinaryOp::Lt),
        Op::Binary(BinaryOp::Le),
        Op::Binary(BinaryOp::Gt),
        Op::Binary(BinaryOp::Ge),
        Op::Binary(BinaryOp::And),
        Op::Binary(BinaryOp::Or),
        Op::Binary(BinaryOp::Contem),
        Op::ShortCircuit {
            target: 19,
            value: false,
        },
        Op::ShortCircuit {
            target: 19,
            value: true,
        },
        Op::Call {
            argc: u32::MAX,
            name: u32::MAX,
        },
    ];
    for (tag, op) in cases.into_iter().enumerate() {
        let mut w = Writer {
            bytes: vec![],
            items: MAX_ITEMS,
        };
        w.op(op).unwrap();
        assert_eq!(
            w.bytes[0], tag as u8,
            "EXB v1 opcode assignments are pinned"
        );
        let mut r = Reader::new(&w.bytes);
        let decoded = r.op().unwrap();
        assert_eq!(decoded, op);
        assert_eq!(r.pos, w.bytes.len());
        let mut out = Writer {
            bytes: vec![],
            items: MAX_ITEMS,
        };
        out.op(decoded).unwrap();
        assert_eq!(out.bytes, w.bytes);
    }
    let mut r = Reader::new(&[255]);
    assert!(r.op().is_err());
}

#[test]
fn runtime_layout_remains_unpacked_and_file_limits_are_bounded() {
    assert_eq!(std::mem::size_of::<Instruction>(), 16);
    assert!(load_exb(&vec![0; EXB_MAX_BYTES + 1]).is_err());
    let mut bytes = compile_exb("1", "test.lep").unwrap();
    bytes.push(0);
    let len = (bytes.len() - HEADER) as u32;
    bytes[8..12].copy_from_slice(&len.to_le_bytes());
    let hash = checksum(&bytes[HEADER..]);
    bytes[12..20].copy_from_slice(&hash.to_le_bytes());
    match load_exb(&bytes) {
        Err(e) => assert!(e.message.contains("dados após")),
        Ok(_) => panic!("trailing byte accepted"),
    }
}

#[test]
fn version_one_golden_file_remains_compatible() {
    let hex = "4558420001000300d7000000e56a91e5538f756c0a000000676f6c64656e2e6c65700a000000657363726576612831290100000001000000000000000a000000040000000100000001000000000000000a0000000100000001000000000000000700000001000000090000000800000009000000000000000000000000000000000000000100000007000000657363726576610100000000000000000100000000000000000000000000000000000000000000000700000008000000001b000000000100000019000000000200000047010000000000000000000000010000000000030000000703000000";
    let bytes: Vec<u8> = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    let program = load_exb(&bytes).unwrap();
    assert_eq!(program.source_name(), "golden.lep");
    assert_eq!(forged(&program), bytes);
    let result = crate::runtime::run_compiled_with_options(
        &program,
        "golden.exb",
        "",
        ExecutionOptions {
            backend: ExecutionBackend::Bytecode,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.stdout, "1\n");
}

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
        "test.lepc",
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
        "test.lepc",
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
fn arbitrary_mutations_do_not_panic() {
    let original = compile_exb("f = funcao(x) { x + 1 }\nescreva(f(2))", "test.lep").unwrap();
    for i in 0..512 {
        let mut bytes = original.clone();
        let pos = HEADER + (i * 7919) % (bytes.len() - HEADER);
        bytes[pos] ^= (i as u8).wrapping_mul(17) | 1;
        let hash = checksum(&bytes[HEADER..]);
        bytes[12..20].copy_from_slice(&hash.to_le_bytes());
        let loaded = std::panic::catch_unwind(|| load_exb(&bytes));
        assert!(loaded.is_ok(), "mutation {i}, byte {pos}");
        let Ok(program) = loaded.unwrap() else {
            continue;
        };
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::runtime::run_compiled_with_options(
                &program,
                "mutacao.lepc",
                "",
                crate::runtime::ExecutionOptions {
                    backend: ExecutionBackend::Bytecode,
                    time_limit: Some(Duration::from_millis(1)),
                    ..Default::default()
                },
            )
        }));
        assert!(ran.is_ok(), "mutation {i}, byte {pos} panicked");
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
        Op::Garanta { cond: u32::MAX },
    ];
    let mut tag = 0u8;
    for op in cases {
        if tag == 66 {
            tag = 68;
        }
        let mut w = Writer {
            bytes: vec![],
            items: MAX_ITEMS,
        };
        w.op(op).unwrap();
        assert_eq!(w.bytes[0], tag, "EXB v2 opcode assignments are pinned");
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
        tag += 1;
    }
    assert_eq!(tag, 73);
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
fn version_two_golden_file_remains_compatible() {
    let hex = "4558420002000300d7000000e56a91e5538f756c0a000000676f6c64656e2e6c65700a000000657363726576612831290100000001000000000000000a000000040000000100000001000000000000000a0000000100000001000000000000000700000001000000090000000800000009000000000000000000000000000000000000000100000007000000657363726576610100000000000000000100000000000000000000000000000000000000000000000700000008000000001b000000000100000019000000000200000047010000000000000000000000010000000000030000000703000000";
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
        "golden.lepc",
        "",
        ExecutionOptions {
            backend: ExecutionBackend::Bytecode,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.stdout, "1\n");
}

#[test]
fn version_one_is_rejected() {
    let mut bytes = compile_exb("escreva(1)", "test.lep").unwrap();
    assert_eq!(&bytes[4..6], &EXB_VERSION.to_le_bytes());
    bytes[4] = 1;
    match load_exb(&bytes) {
        Err(err) => assert!(err.message.contains("versão EXB não suportada"), "{err}"),
        Ok(_) => panic!("versão 1 foi aceita"),
    }
}

#[test]
fn opcodes_66_and_67_are_rejected() {
    for tag in [66u8, 67] {
        let err = Reader::new(&[tag]).op().unwrap_err();
        assert!(err.message.contains("opcode EXB inválido"), "{err}");
    }
    for op in [Op::Binary(BinaryOp::And), Op::Binary(BinaryOp::Or)] {
        let mut w = Writer {
            bytes: vec![],
            items: MAX_ITEMS,
        };
        let err = w.op(op).unwrap_err();
        assert!(err.message.contains("opcode EXB inválido"), "{err}");
    }

    let bytes = compile_exb("escreva(1 + 2)", "test.lep").unwrap();
    let program = load_exb(&bytes).unwrap();
    let span = program
        .code
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.op, Op::Binary(BinaryOp::Add)))
        .expect("soma")
        .span;
    let mut needle = vec![55u8];
    needle.extend_from_slice(&span.to_le_bytes());
    let hits: Vec<usize> = bytes[HEADER..]
        .windows(needle.len())
        .enumerate()
        .filter(|(_, window)| *window == needle.as_slice())
        .map(|(index, _)| index)
        .collect();
    assert_eq!(hits.len(), 1, "uma única soma no fio");
    for tag in [66u8, 67] {
        let mut broken = bytes.clone();
        broken[HEADER + hits[0]] = tag;
        let hash = checksum(&broken[HEADER..]);
        broken[12..20].copy_from_slice(&hash.to_le_bytes());
        let loaded = std::panic::catch_unwind(|| load_exb(&broken));
        assert!(loaded.is_ok(), "tag {tag} panicked in the loader");
        match loaded.unwrap() {
            Err(err) => assert!(err.message.contains("opcode EXB inválido"), "{err}"),
            Ok(_) => panic!("tag {tag} foi aceita"),
        }
        let ran = std::panic::catch_unwind(|| {
            crate::runtime::run_exb_with_options(
                &broken,
                "test.lepc",
                "",
                ExecutionOptions {
                    backend: ExecutionBackend::Bytecode,
                    time_limit: Some(Duration::from_millis(1)),
                    ..Default::default()
                },
            )
        });
        assert!(ran.is_ok(), "tag {tag} panicked when run");
        assert!(ran.unwrap().is_err());
    }
}

#[test]
fn loop_kind_one_is_exclusive_range_and_slice_builds_faixa() {
    let lugar = Span::new(1, 1, 0, 1);
    let ops = [
        Op::Constant(0),
        Op::Constant(1),
        Op::LoopEnter(0),
        Op::LoopNext(0),
        Op::LoopBody,
        Op::Checkpoint,
        Op::Load(1),
        Op::Load(0),
        Op::Call { argc: 1, name: 1 },
        Op::Drop,
        Op::Jump(3),
        Op::LoopEnd,
        Op::Load(1),
        Op::Constant(0),
        Op::Constant(2),
        Op::Slice(true),
        Op::Call { argc: 1, name: 1 },
        Op::Drop,
        Op::Nada,
        Op::Finish,
    ];
    let program = CompiledProgram {
        source_name: "faixa.lep".into(),
        source: Rc::from("faixa"),
        span: lugar,
        code: Rc::new(Chunk {
            parameter_slots: Rc::new(HashMap::new()),
            instructions: ops
                .into_iter()
                .map(|op| Instruction { op, span: 0 })
                .collect(),
            spans: vec![lugar],
            names: vec!["i".into(), "escreva".into()],
            constants: vec![
                Value::Numero(Numero::from_i64(1)),
                Value::Numero(Numero::from_i64(3)),
                Value::Numero(Numero::from_i64(4)),
            ],
            functions: vec![],
            imports: vec![],
            loops: vec![LoopSpec {
                kind: LoopKind::Range,
                var: Some(0),
                first_span: lugar,
                second_span: lugar,
                next: 3,
                exit: 11,
            }],
        }),
    };
    let bytes = forged(&program);
    assert_eq!(loop_kind_byte(&bytes), 1, "tipo 1 é a faixa exclusiva");
    let loaded = load_exb(&bytes).unwrap();
    assert!(matches!(loaded.code.loops[0].kind, LoopKind::Range));
    let result = crate::runtime::run_compiled_with_options(
        &loaded,
        "faixa.lepc",
        "",
        ExecutionOptions {
            backend: ExecutionBackend::Bytecode,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.stdout, "1\n2\n1..4\n");
}

fn loop_kind_byte(bytes: &[u8]) -> u8 {
    let mut r = Reader::new(&bytes[HEADER..]);
    r.string().unwrap();
    r.string().unwrap();
    r.span().unwrap();
    r.vector(16, Reader::span).unwrap();
    r.vector(4, Reader::string).unwrap();
    let constants = r.u32().unwrap();
    for _ in 0..constants {
        match r.u8().unwrap() {
            0 => {
                r.take(16).unwrap();
            }
            1 => {
                r.string().unwrap();
            }
            other => panic!("constante {other}"),
        }
    }
    assert_eq!(r.u32().unwrap(), 0, "funções");
    assert_eq!(r.u32().unwrap(), 0, "importações");
    assert_eq!(r.u32().unwrap(), 1, "laços");
    r.u8().unwrap()
}

#[test]
fn short_circuit_binary_ops_are_errors_not_panics() {
    for op in [BinaryOp::And, BinaryOp::Or] {
        let mut program = fixture("escreva(1 + 2)");
        let code = Rc::get_mut(&mut program.code).unwrap();
        let slot = code
            .instructions
            .iter_mut()
            .find(|instruction| matches!(instruction.op, Op::Binary(BinaryOp::Add)))
            .expect("soma");
        slot.op = Op::Binary(op);
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::runtime::run_compiled_with_options(
                &program,
                "test.lepc",
                "",
                ExecutionOptions {
                    backend: ExecutionBackend::Bytecode,
                    ..Default::default()
                },
            )
        }));
        let err = ran.expect("e/ou panicked").unwrap_err();
        assert!(err.message.contains("não são instruções binárias"), "{err}");
    }
}

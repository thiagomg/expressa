//! Stable EXB v1 opcode assignments. Do not renumber without a format version change.
use super::*;
impl Writer {
    pub(super) fn op(&mut self, op: Op) -> Result<()> {
        match op {
            Op::Nada => self.u8(0),
            Op::Drop => self.u8(1),
            Op::SetLast => self.u8(2),
            Op::Pair => self.u8(3),
            Op::Index => self.u8(4),
            Op::Index2 => self.u8(5),
            Op::Return => self.u8(6),
            Op::Finish => self.u8(7),
            Op::Checkpoint => self.u8(8),
            Op::EnterScope => self.u8(9),
            Op::LeaveScope => self.u8(10),
            Op::ParentScope => self.u8(11),
            Op::RestoreScope => self.u8(12),
            Op::CheckBool => self.u8(13),
            Op::CheckInt => self.u8(14),
            Op::EndTry => self.u8(15),
            Op::LoopBody => self.u8(16),
            Op::LoopEnd => self.u8(17),
            Op::Break => self.u8(18),
            Op::Continue => self.u8(19),
            Op::ReceiverIndex => self.u8(20),
            Op::ReceiverIndex2 => self.u8(21),
            Op::MatrixDestination => self.u8(22),
            Op::ReadDestination => self.u8(23),
            Op::WriteDestination => self.u8(24),
            Op::Constant(value) => {
                self.u8(25)?;
                self.u32(value)
            }
            Op::BadNumber(value) => {
                self.u8(26)?;
                self.u32(value)
            }
            Op::Load(value) => {
                self.u8(27)?;
                self.u32(value)
            }
            Op::Store(value) => {
                self.u8(28)?;
                self.u32(value)
            }
            Op::LoadLocal(value) => {
                self.u8(29)?;
                self.u32(value)
            }
            Op::StoreLocal(value) => {
                self.u8(30)?;
                self.u32(value)
            }
            Op::List(value) => {
                self.u8(31)?;
                self.u32(value)
            }
            Op::Field(value) => {
                self.u8(32)?;
                self.u32(value)
            }
            Op::MapField(value) => {
                self.u8(33)?;
                self.u32(value)
            }
            Op::Closure(value) => {
                self.u8(34)?;
                self.u32(value)
            }
            Op::Jump(value) => {
                self.u8(35)?;
                self.u32(value)
            }
            Op::BranchFalse(value) => {
                self.u8(36)?;
                self.u32(value)
            }
            Op::Try(value) => {
                self.u8(37)?;
                self.u32(value)
            }
            Op::Import(value) => {
                self.u8(38)?;
                self.u32(value)
            }
            Op::LoopEnter(value) => {
                self.u8(39)?;
                self.u32(value)
            }
            Op::LoopNext(value) => {
                self.u8(40)?;
                self.u32(value)
            }
            Op::NameDestination(value) => {
                self.u8(41)?;
                self.u32(value)
            }
            Op::ReceiverName(value) => {
                self.u8(42)?;
                self.u32(value)
            }
            Op::ReceiverMap(value) => {
                self.u8(43)?;
                self.u32(value)
            }
            Op::MapDestination(value) => {
                self.u8(44)?;
                self.u32(value)
            }
            Op::Bool(false) => self.u8(45),
            Op::Bool(true) => self.u8(46),
            Op::Slice(false) => self.u8(47),
            Op::Slice(true) => self.u8(48),
            Op::ReceiverReadOnly(false) => self.u8(49),
            Op::ReceiverReadOnly(true) => self.u8(50),
            Op::IndexDestination(false) => self.u8(51),
            Op::IndexDestination(true) => self.u8(52),
            Op::Unary(UnaryOp::Neg) => self.u8(53),
            Op::Unary(UnaryOp::Not) => self.u8(54),
            Op::Binary(BinaryOp::Add) => self.u8(55),
            Op::Binary(BinaryOp::Sub) => self.u8(56),
            Op::Binary(BinaryOp::Mul) => self.u8(57),
            Op::Binary(BinaryOp::Div) => self.u8(58),
            Op::Binary(BinaryOp::Rem) => self.u8(59),
            Op::Binary(BinaryOp::Eq) => self.u8(60),
            Op::Binary(BinaryOp::Ne) => self.u8(61),
            Op::Binary(BinaryOp::Lt) => self.u8(62),
            Op::Binary(BinaryOp::Le) => self.u8(63),
            Op::Binary(BinaryOp::Gt) => self.u8(64),
            Op::Binary(BinaryOp::Ge) => self.u8(65),
            Op::Binary(BinaryOp::And) => self.u8(66),
            Op::Binary(BinaryOp::Or) => self.u8(67),
            Op::Binary(BinaryOp::Contem) => self.u8(68),
            Op::ShortCircuit {
                target,
                value: false,
            } => {
                self.u8(69)?;
                self.u32(target)
            }
            Op::ShortCircuit {
                target,
                value: true,
            } => {
                self.u8(70)?;
                self.u32(target)
            }
            Op::Call { argc, name } => {
                self.u8(71)?;
                self.u32(argc)?;
                self.u32(name)
            }
        }
    }
}
impl Reader<'_> {
    pub(super) fn op(&mut self) -> Result<Op> {
        Ok(match self.u8()? {
            0 => Op::Nada,
            1 => Op::Drop,
            2 => Op::SetLast,
            3 => Op::Pair,
            4 => Op::Index,
            5 => Op::Index2,
            6 => Op::Return,
            7 => Op::Finish,
            8 => Op::Checkpoint,
            9 => Op::EnterScope,
            10 => Op::LeaveScope,
            11 => Op::ParentScope,
            12 => Op::RestoreScope,
            13 => Op::CheckBool,
            14 => Op::CheckInt,
            15 => Op::EndTry,
            16 => Op::LoopBody,
            17 => Op::LoopEnd,
            18 => Op::Break,
            19 => Op::Continue,
            20 => Op::ReceiverIndex,
            21 => Op::ReceiverIndex2,
            22 => Op::MatrixDestination,
            23 => Op::ReadDestination,
            24 => Op::WriteDestination,
            25 => {
                let operand = self.u32()?;
                Op::Constant(operand)
            }
            26 => {
                let operand = self.u32()?;
                Op::BadNumber(operand)
            }
            27 => {
                let operand = self.u32()?;
                Op::Load(operand)
            }
            28 => {
                let operand = self.u32()?;
                Op::Store(operand)
            }
            29 => {
                let operand = self.u32()?;
                Op::LoadLocal(operand)
            }
            30 => {
                let operand = self.u32()?;
                Op::StoreLocal(operand)
            }
            31 => {
                let operand = self.u32()?;
                Op::List(operand)
            }
            32 => {
                let operand = self.u32()?;
                Op::Field(operand)
            }
            33 => {
                let operand = self.u32()?;
                Op::MapField(operand)
            }
            34 => {
                let operand = self.u32()?;
                Op::Closure(operand)
            }
            35 => {
                let operand = self.u32()?;
                Op::Jump(operand)
            }
            36 => {
                let operand = self.u32()?;
                Op::BranchFalse(operand)
            }
            37 => {
                let operand = self.u32()?;
                Op::Try(operand)
            }
            38 => {
                let operand = self.u32()?;
                Op::Import(operand)
            }
            39 => {
                let operand = self.u32()?;
                Op::LoopEnter(operand)
            }
            40 => {
                let operand = self.u32()?;
                Op::LoopNext(operand)
            }
            41 => {
                let operand = self.u32()?;
                Op::NameDestination(operand)
            }
            42 => {
                let operand = self.u32()?;
                Op::ReceiverName(operand)
            }
            43 => {
                let operand = self.u32()?;
                Op::ReceiverMap(operand)
            }
            44 => {
                let operand = self.u32()?;
                Op::MapDestination(operand)
            }
            45 => Op::Bool(false),
            46 => Op::Bool(true),
            47 => Op::Slice(false),
            48 => Op::Slice(true),
            49 => Op::ReceiverReadOnly(false),
            50 => Op::ReceiverReadOnly(true),
            51 => Op::IndexDestination(false),
            52 => Op::IndexDestination(true),
            53 => Op::Unary(UnaryOp::Neg),
            54 => Op::Unary(UnaryOp::Not),
            55 => Op::Binary(BinaryOp::Add),
            56 => Op::Binary(BinaryOp::Sub),
            57 => Op::Binary(BinaryOp::Mul),
            58 => Op::Binary(BinaryOp::Div),
            59 => Op::Binary(BinaryOp::Rem),
            60 => Op::Binary(BinaryOp::Eq),
            61 => Op::Binary(BinaryOp::Ne),
            62 => Op::Binary(BinaryOp::Lt),
            63 => Op::Binary(BinaryOp::Le),
            64 => Op::Binary(BinaryOp::Gt),
            65 => Op::Binary(BinaryOp::Ge),
            66 => Op::Binary(BinaryOp::And),
            67 => Op::Binary(BinaryOp::Or),
            68 => Op::Binary(BinaryOp::Contem),
            69 => {
                let operand = self.u32()?;
                Op::ShortCircuit {
                    target: operand,
                    value: false,
                }
            }
            70 => {
                let operand = self.u32()?;
                Op::ShortCircuit {
                    target: operand,
                    value: true,
                }
            }
            71 => Op::Call {
                argc: self.u32()?,
                name: self.u32()?,
            },
            _ => return Err(self.err("opcode EXB inválido")),
        })
    }
}

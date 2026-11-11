//! Bytecode compilation and execution. No instruction retains an AST expression.
mod compiler;
mod execute;

use super::super::value::FunctionBody;
use super::assignment::Destination;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionBackend {
    Ast,
    #[default]
    Bytecode,
}

impl ExecutionBackend {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ast" => Some(Self::Ast),
            "bytecode" => Some(Self::Bytecode),
            _ => None,
        }
    }
}

pub(super) fn default_backend() -> ExecutionBackend {
    std::env::var("EXPRESSA_BACKEND")
        .ok()
        .and_then(|s| ExecutionBackend::from_name(&s))
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
enum Op {
    Nada,
    Bool(bool),
    Constant(u32),
    BadNumber(u32),
    Load(u32),
    Store(u32),
    LoadLocal(u32),
    StoreLocal(u32),
    Drop,
    SetLast,
    List(u32),
    Pair,
    Unary(UnaryOp),
    Binary(BinaryOp),
    Field(u32),
    MapField(u32),
    Index,
    Index2,
    Slice(bool),
    Closure(u32),
    Call { argc: u32, name: u32 },
    Return,
    Finish,
    Checkpoint,
    EnterScope,
    LeaveScope,
    ParentScope,
    RestoreScope,
    Jump(u32),
    BranchFalse(u32),
    ShortCircuit { target: u32, value: bool },
    CheckBool,
    CheckInt,
    Try(u32),
    EndTry,
    Import(u32),
    LoopEnter(u32),
    LoopNext(u32),
    LoopBody,
    LoopEnd,
    Break,
    Continue,
    NameDestination(u32),
    ReceiverName(u32),
    ReceiverIndex,
    ReceiverIndex2,
    ReceiverMap(u32),
    ReceiverReadOnly(bool),
    IndexDestination(bool),
    MatrixDestination,
    MapDestination(u32),
    ReadDestination,
    WriteDestination,
}

#[derive(Clone, Copy)]
struct Instruction {
    op: Op,
    span: u32,
}

pub struct Chunk {
    parameter_slots: Rc<HashMap<String, u32>>,
    instructions: Vec<Instruction>,
    spans: Vec<Span>,
    names: Vec<String>,
    constants: Vec<Value>,
    functions: Vec<Prototype>,
    imports: Vec<Import>,
    loops: Vec<LoopSpec>,
}

impl Chunk {
    pub(super) fn bind_parameters(&self, env: &mut Env, args: impl Iterator<Item = Value>) {
        if !self.parameter_slots.is_empty() {
            env.vars
                .bind_parameters(Rc::clone(&self.parameter_slots), args.collect());
        }
    }
}

struct Prototype {
    params: Vec<Param>,
    code: Rc<Chunk>,
    span: Span,
    doc: Option<String>,
}
#[derive(Clone, Copy)]
enum LoopKind {
    Repeat,
    Range,
    Items,
    While,
}
struct LoopSpec {
    kind: LoopKind,
    var: Option<u32>,
    first_span: Span,
    second_span: Span,
    exit: u32,
    next: u32,
}

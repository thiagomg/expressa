//! EXB v1: portable file encoding, independent from the VM's in-memory Op enum.
mod opcodes;
#[cfg(test)]
mod tests;
mod verify;

use super::*;
use std::fmt;
use std::io::Read;

pub const EXB_VERSION: u16 = 1;
pub const EXB_MAX_BYTES: usize = 16 * 1024 * 1024;
const MAGIC: &[u8; 4] = b"EXB\0";
const FLAGS: u16 = 3; // decimal96 constants and embedded debug source
const HEADER: usize = 20;
const MAX_ITEMS: usize = 250_000;
const MAX_DEPTH: usize = 64;
const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExbError {
    pub offset: usize,
    pub message: String,
}
impl fmt::Display for ExbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EXB: {} (byte {})", self.message, self.offset)
    }
}
impl std::error::Error for ExbError {}
fn error(message: impl Into<String>) -> ExbError {
    ExbError {
        offset: 0,
        message: message.into(),
    }
}
type Result<T> = std::result::Result<T, ExbError>;

/// Immutable, validated module. Loading does not parse or execute embedded source.
#[derive(Clone)]
pub struct CompiledProgram {
    pub(in crate::runtime::eval) source_name: String,
    pub(in crate::runtime::eval) source: Rc<str>,
    pub(in crate::runtime::eval) span: Span,
    pub(in crate::runtime::eval) code: Rc<Chunk>,
}
impl CompiledProgram {
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
    pub fn source(&self) -> &str {
        &self.source
    }
}

pub fn compile_exb(source: &str, source_name: &str) -> Result<Vec<u8>> {
    if source.len() > EXB_MAX_BYTES {
        return Err(error("fonte excede o limite EXB de 16 MiB"));
    }
    let program = parse(source).map_err(|e| ExbError {
        offset: e.span.start,
        message: format!("sintaxe em {source_name}: {} ({})", e.message, e.span),
    })?;
    let compiled = CompiledProgram {
        source_name: source_name.into(),
        source: Rc::from(source),
        span: program.span,
        code: compiler::module(&program.items),
    };
    verify::program(&compiled)?;
    let mut w = Writer {
        bytes: Vec::new(),
        items: MAX_ITEMS,
    };
    w.string(source_name)?;
    w.string(source)?;
    w.span(compiled.span)?;
    w.chunk(&compiled.code, 0)?;
    if w.bytes.len() > EXB_MAX_BYTES - HEADER {
        return Err(error("arquivo EXB excede 16 MiB"));
    }
    let mut result = Vec::with_capacity(HEADER + w.bytes.len());
    result.extend_from_slice(MAGIC);
    result.extend_from_slice(&EXB_VERSION.to_le_bytes());
    result.extend_from_slice(&FLAGS.to_le_bytes());
    result.extend_from_slice(&(w.bytes.len() as u32).to_le_bytes());
    result.extend_from_slice(&checksum(&w.bytes).to_le_bytes());
    result.extend(w.bytes);
    Ok(result)
}

pub fn load_exb(bytes: &[u8]) -> Result<CompiledProgram> {
    if bytes.len() > EXB_MAX_BYTES {
        return Err(error("arquivo EXB excede 16 MiB"));
    }
    let mut r = Reader::new(bytes);
    if r.take(4)? != MAGIC {
        return Err(r.err("assinatura EXB inválida"));
    }
    if r.u16()? != EXB_VERSION {
        return Err(r.err("versão EXB não suportada"));
    }
    if r.u16()? != FLAGS {
        return Err(r.err("recursos EXB não suportados"));
    }
    let length = r.u32()? as usize;
    let digest = r.u64()?;
    if length != bytes.len() - HEADER {
        return Err(r.err("tamanho EXB inconsistente"));
    }
    if checksum(&bytes[HEADER..]) != digest {
        return Err(r.err("checksum EXB inválido"));
    }
    let source_name = r.string()?;
    let source = Rc::from(r.string()?);
    let span = r.span()?;
    let code = Rc::new(r.chunk(&[], 0)?);
    if r.pos != bytes.len() {
        return Err(r.err("dados após o módulo EXB"));
    }
    let program = CompiledProgram {
        source_name,
        source,
        span,
        code,
    };
    verify::program(&program)?;
    Ok(program)
}

/// Bounded filesystem read; does not allocate based on an unchecked file length.
pub fn read_exb(path: &Path) -> Result<CompiledProgram> {
    let file = std::fs::File::open(path).map_err(|e| error(format!("{}: {e}", path.display())))?;
    let mut bytes = Vec::new();
    file.take(EXB_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| error(format!("{}: {e}", path.display())))?;
    load_exb(&bytes)
}
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

struct Writer {
    bytes: Vec<u8>,
    items: usize,
}
impl Writer {
    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > (EXB_MAX_BYTES - HEADER).saturating_sub(self.bytes.len()) {
            return Err(error("arquivo EXB excede 16 MiB"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn u8(&mut self, n: u8) -> Result<()> {
        self.raw(&[n])
    }
    fn u32(&mut self, n: u32) -> Result<()> {
        self.raw(&n.to_le_bytes())
    }
    fn count(&mut self, n: usize) -> Result<()> {
        self.items = self
            .items
            .checked_sub(n)
            .ok_or_else(|| error("limite de elementos EXB excedido"))?;
        self.u32(u32::try_from(n).map_err(|_| error("índice fora do intervalo EXB"))?)
    }
    fn string(&mut self, text: &str) -> Result<()> {
        self.u32(u32::try_from(text.len()).map_err(|_| error("texto EXB longo demais"))?)?;
        self.raw(text.as_bytes())
    }
    fn optional(&mut self, text: &Option<String>) -> Result<()> {
        self.u8(u8::from(text.is_some()))?;
        if let Some(text) = text {
            self.string(text)?;
        }
        Ok(())
    }
    fn span(&mut self, span: Span) -> Result<()> {
        self.u32(span.line)?;
        self.u32(span.col)?;
        self.u32(u32::try_from(span.start).map_err(|_| error("posição EXB fora do intervalo"))?)?;
        self.u32(u32::try_from(span.end).map_err(|_| error("posição EXB fora do intervalo"))?)
    }
    fn chunk(&mut self, code: &Chunk, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(error("funções EXB aninhadas demais"));
        }
        self.count(code.spans.len())?;
        for s in &code.spans {
            self.span(*s)?;
        }
        self.count(code.names.len())?;
        for n in &code.names {
            self.string(n)?;
        }
        self.count(code.constants.len())?;
        for v in &code.constants {
            match v {
                Value::Numero(n) => {
                    self.u8(0)?;
                    self.raw(&n.exb_bytes())?;
                }
                Value::Texto(s) => {
                    self.u8(1)?;
                    self.string(s)?;
                }
                _ => return Err(error("constante não suportada pelo EXB")),
            }
        }
        self.count(code.functions.len())?;
        for f in &code.functions {
            self.count(f.params.len())?;
            for p in &f.params {
                self.string(&p.name)?;
                self.span(p.span)?;
            }
            self.optional(&f.doc)?;
            self.span(f.span)?;
            self.chunk(&f.code, depth + 1)?;
        }
        self.count(code.imports.len())?;
        for i in &code.imports {
            self.optional(&i.alias)?;
            self.string(&i.path)?;
            self.optional(&i.doc)?;
            self.span(i.span)?;
        }
        self.count(code.loops.len())?;
        for l in &code.loops {
            self.u8(match l.kind {
                LoopKind::Repeat => 0,
                LoopKind::Range => 1,
                LoopKind::Items => 2,
                LoopKind::While => 3,
            })?;
            self.u8(u8::from(l.var.is_some()))?;
            if let Some(v) = l.var {
                self.u32(v)?;
            }
            self.span(l.first_span)?;
            self.span(l.second_span)?;
            self.u32(l.exit)?;
            self.u32(l.next)?;
        }
        self.count(code.instructions.len())?;
        for i in &code.instructions {
            self.op(i.op)?;
            self.u32(i.span)?;
        }
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    items: usize,
    budget: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            items: MAX_ITEMS,
            budget: MAX_DECODED_BYTES,
        }
    }
    fn err(&self, text: &str) -> ExbError {
        ExbError {
            offset: self.pos,
            message: text.into(),
        }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&n| n <= self.bytes.len())
            .ok_or_else(|| self.err("arquivo EXB truncado"))?;
        let result = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(result)
    }
    fn claim(&mut self, n: usize) -> Result<()> {
        self.budget = self
            .budget
            .checked_sub(n)
            .ok_or_else(|| self.err("orçamento de decodificação EXB excedido"))?;
        Ok(())
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn boolean(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(self.err("booleano EXB inválido")),
        }
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<String> {
        let len = self.u32()? as usize;
        self.claim(len)?;
        let bytes = self.take(len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|_| self.err("texto EXB não é UTF-8"))
    }
    fn optional(&mut self) -> Result<Option<String>> {
        if self.boolean()? {
            Ok(Some(self.string()?))
        } else {
            Ok(None)
        }
    }
    fn span(&mut self) -> Result<Span> {
        let line = self.u32()?;
        let col = self.u32()?;
        let start = self.u32()? as usize;
        let end = self.u32()? as usize;
        if start > end {
            return Err(self.err("intervalo de posição EXB inválido"));
        }
        Ok(Span::new(line, col, start, end))
    }
    fn vector<T>(
        &mut self,
        minimum: usize,
        mut read: impl FnMut(&mut Self) -> Result<T>,
    ) -> Result<Vec<T>> {
        let n = self.u32()? as usize;
        if n > (self.bytes.len() - self.pos) / minimum {
            return Err(self.err("contagem EXB incompatível com o arquivo"));
        }
        self.items = self
            .items
            .checked_sub(n)
            .ok_or_else(|| self.err("limite de elementos EXB excedido"))?;
        self.claim(
            n.checked_mul(std::mem::size_of::<T>())
                .ok_or_else(|| self.err("tamanho EXB fora do intervalo"))?,
        )?;
        let mut out = Vec::new();
        out.try_reserve_exact(n)
            .map_err(|_| self.err("memória insuficiente para EXB"))?;
        for _ in 0..n {
            out.push(read(self)?);
        }
        Ok(out)
    }
    fn chunk(&mut self, params: &[Param], depth: usize) -> Result<Chunk> {
        if depth > MAX_DEPTH {
            return Err(self.err("funções EXB aninhadas demais"));
        }
        self.claim(
            std::mem::size_of::<Chunk>()
                + std::mem::size_of::<HashMap<String, u32>>()
                + 4 * std::mem::size_of::<usize>(),
        )?;
        let spans = self.vector(16, Self::span)?;
        let names = self.vector(4, Self::string)?;
        let constants = self.vector(1, |r| match r.u8()? {
            0 => {
                let bytes = r.take(16)?.try_into().unwrap();
                Numero::from_exb_bytes(bytes)
                    .map(Value::Numero)
                    .ok_or_else(|| r.err("decimal EXB inválido"))
            }
            1 => Ok(Value::Texto(r.string()?)),
            _ => Err(r.err("tipo de constante EXB inválido")),
        })?;
        let functions = self.vector(1, |r| {
            let params = r.vector(20, |r| {
                Ok(Param {
                    name: r.string()?,
                    span: r.span()?,
                })
            })?;
            let doc = r.optional()?;
            let span = r.span()?;
            let code = Rc::new(r.chunk(&params, depth + 1)?);
            Ok(Prototype {
                params,
                code,
                span,
                doc,
            })
        })?;
        let imports = self.vector(1, |r| {
            Ok(Import {
                alias: r.optional()?,
                path: r.string()?,
                doc: r.optional()?,
                span: r.span()?,
            })
        })?;
        let loops = self.vector(1, |r| {
            let kind = match r.u8()? {
                0 => LoopKind::Repeat,
                1 => LoopKind::Range,
                2 => LoopKind::Items,
                3 => LoopKind::While,
                _ => return Err(r.err("laço EXB inválido")),
            };
            let var = if r.boolean()? { Some(r.u32()?) } else { None };
            Ok(LoopSpec {
                kind,
                var,
                first_span: r.span()?,
                second_span: r.span()?,
                exit: r.u32()?,
                next: r.u32()?,
            })
        })?;
        let instructions = self.vector(5, |r| {
            Ok(Instruction {
                op: r.op()?,
                span: r.u32()?,
            })
        })?;
        self.claim(params.iter().map(|p| p.name.len() + 48).sum())?;
        let parameter_slots = Rc::new(
            params
                .iter()
                .enumerate()
                .map(|(i, p)| (p.name.clone(), i as u32))
                .collect(),
        );
        Ok(Chunk {
            parameter_slots,
            instructions,
            spans,
            names,
            constants,
            functions,
            imports,
            loops,
        })
    }
}

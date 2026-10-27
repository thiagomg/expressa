//! Framed messages between the VM host and `expressa-grafico --ao-vivo`.
//! Length-prefixed (`u32` LE + payload). The pipe buffer is not a size cap.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};

use super::graf::OpGraf;
use super::graf::{TelaGrafica, cor_graf_conhecida};
use super::imagem::Imagem;

pub const MAX_PAYLOAD: u32 = 32 * 1024 * 1024;

const ABRA: u8 = 1;
const CARREGUE: u8 = 2;
const MOSTRE: u8 = 3;
const ESPERE: u8 = 4;
const EVENTO: u8 = 5;

const OP_PONTO: u8 = 1;
const OP_LINHA: u8 = 2;
const OP_CIRCULO: u8 = 3;
const OP_IMAGEM: u8 = 4;
const OP_TEXTO: u8 = 5;

pub const TIPO_NADA: u8 = 0;
pub const TIPO_CLIQUE: u8 = 1;
pub const TIPO_SOLTE: u8 = 2;
#[allow(dead_code)]
pub const TIPO_MOVA: u8 = 3;
pub const TIPO_TECLA: u8 = 4;
pub const TIPO_SAIR: u8 = 5;

#[allow(dead_code)]
pub const BOTAO_NENHUM: u8 = 0;
pub const BOTAO_ESQUERDO: u8 = 1;
#[allow(dead_code)]
pub const BOTAO_DIREITO: u8 = 2;
#[allow(dead_code)]
pub const BOTAO_MEIO: u8 = 3;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Abra {
        id: u64,
        largura: u32,
        altura: u32,
        fundo: String,
    },
    CarregueFigura {
        handle: u32,
        largura: u32,
        altura: u32,
        rgba: Vec<u8>,
    },
    Mostre {
        id: u64,
        ops: Vec<OpMsg>,
    },
    Espere {
        id: u64,
        timeout_ms: i32,
    },
    Evento {
        id: u64,
        tipo: u8,
        x: i32,
        y: i32,
        botao: u8,
        tecla: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpMsg {
    Ponto {
        x: i32,
        y: i32,
        cor: String,
    },
    Linha {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        cor: String,
    },
    Circulo {
        x: i32,
        y: i32,
        r: i32,
        cor: String,
    },
    Imagem {
        handle: u32,
        x: i32,
        y: i32,
        angulo: f64,
    },
    Texto {
        x: i32,
        y: i32,
        tamanho: i32,
        cor: String,
        texto: String,
    },
}

#[derive(Default)]
pub struct FiguraIntern {
    next: u32,
    by_ptr: HashMap<usize, u32>,
    imagens: HashMap<u32, Imagem>,
}

impl FiguraIntern {
    pub fn handle(&mut self, img: &Imagem) -> u32 {
        let p = img.px.as_ptr() as usize;
        if let Some(&h) = self.by_ptr.get(&p) {
            return h;
        }
        self.next += 1;
        let h = self.next;
        self.by_ptr.insert(p, h);
        self.imagens.insert(h, img.clone());
        h
    }

    pub fn get(&self, handle: u32) -> Option<&Imagem> {
        self.imagens.get(&handle)
    }
}

/// `ABRA` (if `abrir`), `CARREGUE_FIGURA` for unseen handles, then `MOSTRE`.
pub fn mensagens_de(
    tela: &TelaGrafica,
    intern: &mut FiguraIntern,
    enviadas: &mut HashSet<u32>,
    abrir: bool,
) -> Vec<Msg> {
    let mut out = Vec::new();
    if abrir {
        out.push(Msg::Abra {
            id: tela.id,
            largura: tela.largura,
            altura: tela.altura,
            fundo: tela.fundo.clone(),
        });
    }
    let mut ops = Vec::new();
    for op in &tela.ops {
        match op {
            OpGraf::Ponto { x, y, cor } => ops.push(OpMsg::Ponto {
                x: *x,
                y: *y,
                cor: cor.clone(),
            }),
            OpGraf::Linha {
                x1,
                y1,
                x2,
                y2,
                cor,
            } => ops.push(OpMsg::Linha {
                x1: *x1,
                y1: *y1,
                x2: *x2,
                y2: *y2,
                cor: cor.clone(),
            }),
            OpGraf::Circulo { x, y, r, cor } => ops.push(OpMsg::Circulo {
                x: *x,
                y: *y,
                r: *r,
                cor: cor.clone(),
            }),
            OpGraf::Imagem {
                x,
                y,
                angulo,
                figura,
            } => {
                let h = intern.handle(figura);
                if enviadas.insert(h) {
                    if let Some(img) = intern.get(h) {
                        out.push(Msg::CarregueFigura {
                            handle: h,
                            largura: img.largura,
                            altura: img.altura,
                            rgba: img.px.to_vec(),
                        });
                    }
                }
                ops.push(OpMsg::Imagem {
                    handle: h,
                    x: *x,
                    y: *y,
                    angulo: *angulo,
                });
            }
            OpGraf::Texto {
                x,
                y,
                tamanho,
                cor,
                texto,
            } => ops.push(OpMsg::Texto {
                x: *x,
                y: *y,
                tamanho: *tamanho,
                cor: cor.clone(),
                texto: texto.clone(),
            }),
        }
    }
    out.push(Msg::Mostre { id: tela.id, ops });
    out
}

pub fn tela_de_mostre(
    id: u64,
    largura: u32,
    altura: u32,
    fundo: &str,
    ops: &[OpMsg],
    figuras: &HashMap<u32, Imagem>,
) -> TelaGrafica {
    let mut out = Vec::new();
    for op in ops {
        match op {
            OpMsg::Ponto { x, y, cor } => out.push(OpGraf::Ponto {
                x: *x,
                y: *y,
                cor: cor_ok(cor),
            }),
            OpMsg::Linha {
                x1,
                y1,
                x2,
                y2,
                cor,
            } => out.push(OpGraf::Linha {
                x1: *x1,
                y1: *y1,
                x2: *x2,
                y2: *y2,
                cor: cor_ok(cor),
            }),
            OpMsg::Circulo { x, y, r, cor } => out.push(OpGraf::Circulo {
                x: *x,
                y: *y,
                r: *r,
                cor: cor_ok(cor),
            }),
            OpMsg::Imagem {
                handle,
                x,
                y,
                angulo,
            } => {
                if let Some(figura) = figuras.get(handle) {
                    out.push(OpGraf::Imagem {
                        x: *x,
                        y: *y,
                        angulo: *angulo,
                        figura: figura.clone(),
                    });
                }
            }
            OpMsg::Texto {
                x,
                y,
                tamanho,
                cor,
                texto,
            } => out.push(OpGraf::Texto {
                x: *x,
                y: *y,
                tamanho: (*tamanho).clamp(1, 256),
                cor: cor_ok(cor),
                texto: texto.clone(),
            }),
        }
    }
    TelaGrafica {
        id,
        largura,
        altura,
        fundo: cor_ok(fundo),
        ops: out,
    }
}

fn cor_ok(nome: &str) -> String {
    if cor_graf_conhecida(nome) {
        nome.to_string()
    } else {
        "preto".into()
    }
}

pub fn write_msg(w: &mut impl Write, msg: &Msg) -> io::Result<()> {
    let mut payload = Vec::new();
    encode(&mut payload, msg);
    let n = payload.len() as u32;
    if n > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mensagem gráfica grande demais",
        ));
    }
    w.write_all(&n.to_le_bytes())?;
    w.write_all(&payload)?;
    w.flush()
}

pub fn read_msg(r: &mut impl Read) -> io::Result<Msg> {
    let mut lenb = [0u8; 4];
    r.read_exact(&mut lenb)?;
    let n = u32::from_le_bytes(lenb);
    if n > MAX_PAYLOAD {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "mensagem gráfica grande demais",
        ));
    }
    let mut payload = vec![0u8; n as usize];
    r.read_exact(&mut payload)?;
    decode(&payload)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "mensagem gráfica inválida"))
}

fn encode(buf: &mut Vec<u8>, msg: &Msg) {
    match msg {
        Msg::Abra {
            id,
            largura,
            altura,
            fundo,
        } => {
            buf.push(ABRA);
            put_u64(buf, *id);
            put_u32(buf, *largura);
            put_u32(buf, *altura);
            put_str(buf, fundo);
        }
        Msg::CarregueFigura {
            handle,
            largura,
            altura,
            rgba,
        } => {
            buf.push(CARREGUE);
            put_u32(buf, *handle);
            put_u32(buf, *largura);
            put_u32(buf, *altura);
            put_u32(buf, rgba.len() as u32);
            buf.extend_from_slice(rgba);
        }
        Msg::Mostre { id, ops } => {
            buf.push(MOSTRE);
            put_u64(buf, *id);
            put_u32(buf, ops.len() as u32);
            for op in ops {
                encode_op(buf, op);
            }
        }
        Msg::Espere { id, timeout_ms } => {
            buf.push(ESPERE);
            put_u64(buf, *id);
            put_i32(buf, *timeout_ms);
        }
        Msg::Evento {
            id,
            tipo,
            x,
            y,
            botao,
            tecla,
        } => {
            buf.push(EVENTO);
            put_u64(buf, *id);
            buf.push(*tipo);
            put_i32(buf, *x);
            put_i32(buf, *y);
            buf.push(*botao);
            put_str(buf, tecla);
        }
    }
}

fn encode_op(buf: &mut Vec<u8>, op: &OpMsg) {
    match op {
        OpMsg::Ponto { x, y, cor } => {
            buf.push(OP_PONTO);
            put_i32(buf, *x);
            put_i32(buf, *y);
            put_str(buf, cor);
        }
        OpMsg::Linha {
            x1,
            y1,
            x2,
            y2,
            cor,
        } => {
            buf.push(OP_LINHA);
            put_i32(buf, *x1);
            put_i32(buf, *y1);
            put_i32(buf, *x2);
            put_i32(buf, *y2);
            put_str(buf, cor);
        }
        OpMsg::Circulo { x, y, r, cor } => {
            buf.push(OP_CIRCULO);
            put_i32(buf, *x);
            put_i32(buf, *y);
            put_i32(buf, *r);
            put_str(buf, cor);
        }
        OpMsg::Imagem {
            handle,
            x,
            y,
            angulo,
        } => {
            buf.push(OP_IMAGEM);
            put_u32(buf, *handle);
            put_i32(buf, *x);
            put_i32(buf, *y);
            buf.extend_from_slice(&angulo.to_le_bytes());
        }
        OpMsg::Texto {
            x,
            y,
            tamanho,
            cor,
            texto,
        } => {
            buf.push(OP_TEXTO);
            put_i32(buf, *x);
            put_i32(buf, *y);
            put_i32(buf, *tamanho);
            put_str(buf, cor);
            put_str(buf, texto);
        }
    }
}

fn decode(p: &[u8]) -> Option<Msg> {
    let mut i = 0;
    let kind = get_u8(p, &mut i)?;
    match kind {
        ABRA => Some(Msg::Abra {
            id: get_u64(p, &mut i)?,
            largura: get_u32(p, &mut i)?,
            altura: get_u32(p, &mut i)?,
            fundo: get_str(p, &mut i)?,
        }),
        CARREGUE => {
            let handle = get_u32(p, &mut i)?;
            let largura = get_u32(p, &mut i)?;
            let altura = get_u32(p, &mut i)?;
            let n = get_u32(p, &mut i)? as usize;
            let rgba = get_bytes(p, &mut i, n)?;
            Some(Msg::CarregueFigura {
                handle,
                largura,
                altura,
                rgba,
            })
        }
        MOSTRE => {
            let id = get_u64(p, &mut i)?;
            let n = get_u32(p, &mut i)? as usize;
            let mut ops = Vec::with_capacity(n);
            for _ in 0..n {
                ops.push(decode_op(p, &mut i)?);
            }
            Some(Msg::Mostre { id, ops })
        }
        ESPERE => Some(Msg::Espere {
            id: get_u64(p, &mut i)?,
            timeout_ms: get_i32(p, &mut i)?,
        }),
        EVENTO => Some(Msg::Evento {
            id: get_u64(p, &mut i)?,
            tipo: get_u8(p, &mut i)?,
            x: get_i32(p, &mut i)?,
            y: get_i32(p, &mut i)?,
            botao: get_u8(p, &mut i)?,
            tecla: get_str(p, &mut i)?,
        }),
        _ => None,
    }
}

fn decode_op(p: &[u8], i: &mut usize) -> Option<OpMsg> {
    match get_u8(p, i)? {
        OP_PONTO => Some(OpMsg::Ponto {
            x: get_i32(p, i)?,
            y: get_i32(p, i)?,
            cor: get_str(p, i)?,
        }),
        OP_LINHA => Some(OpMsg::Linha {
            x1: get_i32(p, i)?,
            y1: get_i32(p, i)?,
            x2: get_i32(p, i)?,
            y2: get_i32(p, i)?,
            cor: get_str(p, i)?,
        }),
        OP_CIRCULO => Some(OpMsg::Circulo {
            x: get_i32(p, i)?,
            y: get_i32(p, i)?,
            r: get_i32(p, i)?,
            cor: get_str(p, i)?,
        }),
        OP_IMAGEM => {
            let handle = get_u32(p, i)?;
            let x = get_i32(p, i)?;
            let y = get_i32(p, i)?;
            if *i + 8 > p.len() {
                return None;
            }
            let mut b = [0u8; 8];
            b.copy_from_slice(&p[*i..*i + 8]);
            *i += 8;
            Some(OpMsg::Imagem {
                handle,
                x,
                y,
                angulo: f64::from_le_bytes(b),
            })
        }
        OP_TEXTO => Some(OpMsg::Texto {
            x: get_i32(p, i)?,
            y: get_i32(p, i)?,
            tamanho: get_i32(p, i)?,
            cor: get_str(p, i)?,
            texto: get_str(p, i)?,
        }),
        _ => None,
    }
}

fn put_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn put_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn put_i32(buf: &mut Vec<u8>, v: i32) {
    buf.extend_from_slice(&v.to_le_bytes());
}
fn put_str(buf: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    let n = (b.len().min(65535)) as u16;
    buf.extend_from_slice(&n.to_le_bytes());
    buf.extend_from_slice(&b[..n as usize]);
}

fn get_u8(p: &[u8], i: &mut usize) -> Option<u8> {
    let v = *p.get(*i)?;
    *i += 1;
    Some(v)
}
fn get_u32(p: &[u8], i: &mut usize) -> Option<u32> {
    if *i + 4 > p.len() {
        return None;
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(&p[*i..*i + 4]);
    *i += 4;
    Some(u32::from_le_bytes(b))
}
fn get_u64(p: &[u8], i: &mut usize) -> Option<u64> {
    if *i + 8 > p.len() {
        return None;
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&p[*i..*i + 8]);
    *i += 8;
    Some(u64::from_le_bytes(b))
}
fn get_i32(p: &[u8], i: &mut usize) -> Option<i32> {
    get_u32(p, i).map(|u| u as i32)
}
fn get_str(p: &[u8], i: &mut usize) -> Option<String> {
    if *i + 2 > p.len() {
        return None;
    }
    let n = u16::from_le_bytes([p[*i], p[*i + 1]]) as usize;
    *i += 2;
    let b = get_bytes(p, i, n)?;
    String::from_utf8(b).ok()
}
fn get_bytes(p: &[u8], i: &mut usize, n: usize) -> Option<Vec<u8>> {
    if *i + n > p.len() {
        return None;
    }
    let b = p[*i..*i + n].to_vec();
    *i += n;
    Some(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::graf::OpGraf;
    use std::sync::Arc;

    fn roundtrip(msg: &Msg) {
        let mut buf = Vec::new();
        write_msg(&mut buf, msg).unwrap();
        let got = read_msg(&mut buf.as_slice()).unwrap();
        assert_eq!(*msg, got);
    }

    #[test]
    fn recados_ida_e_volta() {
        roundtrip(&Msg::Abra {
            id: 9,
            largura: 320,
            altura: 200,
            fundo: "branco".into(),
        });
        roundtrip(&Msg::Espere {
            id: 9,
            timeout_ms: 50,
        });
        roundtrip(&Msg::Evento {
            id: 9,
            tipo: TIPO_CLIQUE,
            x: 10,
            y: 20,
            botao: BOTAO_ESQUERDO,
            tecla: String::new(),
        });
        roundtrip(&Msg::Mostre {
            id: 1,
            ops: vec![
                OpMsg::Linha {
                    x1: 0,
                    y1: 0,
                    x2: 7,
                    y2: 7,
                    cor: "azul".into(),
                },
                OpMsg::Imagem {
                    handle: 1,
                    x: 40,
                    y: 100,
                    angulo: 90.0,
                },
                OpMsg::Texto {
                    x: 8,
                    y: 8,
                    tamanho: 8,
                    cor: "preto".into(),
                    texto: "olá".into(),
                },
            ],
        });
    }

    #[test]
    fn figura_carrega_uma_vez() {
        let px: Arc<[u8]> = Arc::from(vec![255, 0, 0, 255]);
        let img = Imagem {
            largura: 1,
            altura: 1,
            px,
        };
        let tela = TelaGrafica {
            id: 3,
            largura: 8,
            altura: 8,
            fundo: "branco".into(),
            ops: vec![OpGraf::Imagem {
                x: 4,
                y: 4,
                angulo: 0.0,
                figura: img.clone(),
            }],
        };
        let mut intern = FiguraIntern::default();
        let mut enviadas = HashSet::new();
        let a = mensagens_de(&tela, &mut intern, &mut enviadas, true);
        assert!(matches!(a[0], Msg::Abra { .. }));
        assert!(matches!(a[1], Msg::CarregueFigura { handle: 1, .. }));
        assert!(matches!(a[2], Msg::Mostre { .. }));
        let tela2 = TelaGrafica {
            ops: vec![OpGraf::Imagem {
                x: 6,
                y: 4,
                angulo: 24.0,
                figura: img,
            }],
            ..tela
        };
        let b = mensagens_de(&tela2, &mut intern, &mut enviadas, false);
        assert_eq!(b.len(), 1);
        match &b[0] {
            Msg::Mostre { ops, .. } => match &ops[0] {
                OpMsg::Imagem {
                    handle, x, angulo, ..
                } => {
                    assert_eq!(*handle, 1);
                    assert_eq!(*x, 6);
                    assert_eq!(*angulo, 24.0);
                }
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }
}

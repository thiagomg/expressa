//! Bounded, ordered control messages and replaceable frames for the live viewer.
//! Taking a batch is atomic: every frame's image loads precede that frame.

use super::graf_proto::Msg;
use super::quadros::QuadrosPendentes;
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

const MAX_CONTROLES: usize = 8;

#[derive(Default)]
struct Estado {
    controles: VecDeque<Msg>,
    quadros: QuadrosPendentes<Msg>,
    fim: bool,
}

#[derive(Clone, Default)]
pub(super) struct CaixaVisor(Arc<(Mutex<Estado>, Condvar)>);

impl CaixaVisor {
    pub fn envie(&self, msg: Msg) -> bool {
        let (mutex, livre) = &*self.0;
        let mut estado = mutex.lock().unwrap();
        if let Msg::Mostre { id, .. } = &msg {
            if estado.fim {
                return false;
            }
            estado.quadros.publique(*id, msg);
            return true;
        }
        while estado.controles.len() == MAX_CONTROLES && !estado.fim {
            estado = livre.wait(estado).unwrap();
        }
        if estado.fim {
            return false;
        }
        estado.controles.push_back(msg);
        true
    }

    pub fn retire(&self) -> (Vec<Msg>, Vec<Msg>, bool) {
        let (mutex, livre) = &*self.0;
        let mut estado = mutex.lock().unwrap();
        let controles = estado.controles.drain(..).collect();
        let quadros = estado.quadros.retire();
        let fim = estado.fim;
        livre.notify_all();
        (controles, quadros, fim)
    }

    pub fn feche(&self) {
        let (mutex, livre) = &*self.0;
        mutex.lock().unwrap().fim = true;
        livre.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::super::graf_proto::OpMsg;
    use super::*;

    fn abra(id: u64) -> Msg {
        Msg::Abra {
            id,
            largura: 16,
            altura: 16,
            tamanho_pixel: 1,
            fundo: "branco".into(),
        }
    }

    #[test]
    fn frame_flood_keeps_controls_ordered_and_eof_preserves_the_last() {
        let inbox = CaixaVisor::default();
        inbox.envie(Msg::Abra {
            id: 1,
            largura: 16,
            altura: 16,
            tamanho_pixel: 1,
            fundo: "branco".into(),
        });
        inbox.envie(Msg::CarregueFigura {
            handle: 7,
            largura: 1,
            altura: 1,
            rgba: vec![255; 4],
        });
        for n in 0..100_000 {
            inbox.envie(Msg::Mostre {
                id: 1,
                ops: vec![OpMsg::Imagem {
                    eixo: [0.0, 0.0],
                    handle: 7,
                    x: n,
                    y: 0,
                    angulo: 0.0,
                }],
            });
        }
        inbox.feche();
        let (controls, frames, eof) = inbox.retire();
        assert!(eof);
        assert_eq!(controls.len(), 2);
        assert!(matches!(controls[0], Msg::Abra { .. }));
        assert!(matches!(controls[1], Msg::CarregueFigura { handle: 7, .. }));
        assert_eq!(frames.len(), 1);
        let Msg::Mostre { ops, .. } = &frames[0] else {
            panic!()
        };
        assert!(matches!(
            ops[0],
            OpMsg::Imagem {
                eixo: [0.0, 0.0],
                handle: 7,
                x: 99_999,
                ..
            }
        ));
        assert!(inbox.retire().1.is_empty());
    }

    #[test]
    fn control_backpressure_is_bounded_and_closing_unblocks_the_reader() {
        let inbox = CaixaVisor::default();
        for _ in 0..MAX_CONTROLES {
            assert!(inbox.envie(abra(1)));
        }
        let writer = inbox.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            tx.send(writer.envie(abra(2))).unwrap();
        });
        assert!(
            rx.recv_timeout(std::time::Duration::from_millis(20))
                .is_err()
        );
        inbox.feche();
        assert!(!rx.recv_timeout(std::time::Duration::from_secs(1)).unwrap());
        thread.join().unwrap();
        assert_eq!(inbox.retire().0.len(), MAX_CONTROLES);
    }
    #[test]
    fn concurrent_replacement_never_exposes_a_frame_before_its_images() {
        let inbox = CaixaVisor::default();
        let writer = inbox.clone();
        let thread = std::thread::spawn(move || {
            for handle in 1..=1000 {
                assert!(writer.envie(Msg::CarregueFigura {
                    handle,
                    largura: 1,
                    altura: 1,
                    rgba: vec![255; 4]
                }));
                assert!(writer.envie(Msg::Mostre {
                    id: 1,
                    ops: vec![OpMsg::Imagem {
                        eixo: [0.0, 0.0],
                        handle,
                        x: 0,
                        y: 0,
                        angulo: 0.0
                    }]
                }));
            }
            writer.feche();
        });
        let mut loaded = std::collections::HashSet::new();
        let mut last = 0;
        loop {
            let (controls, frames, eof) = inbox.retire();
            assert!(controls.len() <= MAX_CONTROLES);
            for control in controls {
                if let Msg::CarregueFigura { handle, .. } = control {
                    loaded.insert(handle);
                }
            }
            assert!(frames.len() <= 1);
            for frame in frames {
                let Msg::Mostre { ops, .. } = frame else {
                    panic!()
                };
                let OpMsg::Imagem { handle, .. } = ops[0] else {
                    panic!()
                };
                assert!(loaded.contains(&handle));
                last = handle;
            }
            if eof {
                break;
            }
            std::thread::yield_now();
        }
        thread.join().unwrap();
        assert_eq!(last, 1000);
    }
}

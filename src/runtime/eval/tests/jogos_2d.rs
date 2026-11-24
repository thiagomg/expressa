use super::*;
use crate::runtime::{
    CapturaEvento, EstadoEntrada, EventoGraf, GrafHost, OpGraf, QuadrosPendentes, TelaGrafica,
};
use std::collections::VecDeque;

fn backends() -> Vec<ExecutionBackend> {
    vec![
        ExecutionBackend::Bytecode,
        #[cfg(feature = "ast")]
        ExecutionBackend::Ast,
    ]
}

fn evaluate(source: &str, backend: ExecutionBackend, host: Option<&mut dyn GrafHost>) -> String {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("exemplos/jogos/explorador.lep");
    let mut out = Vec::new();
    run_configured_with_io(
        source,
        file.to_str().unwrap(),
        Box::new(NoopHook),
        &mut io::Cursor::new(""),
        &mut out,
        &mut Vec::new(),
        ExecutionOptions {
            backend,
            time_limit: Some(Duration::from_secs(5)),
            ..Default::default()
        },
        None,
        None,
        false,
        host,
        &[],
    )
    .unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn collision_sliding_diagonal_speed_collectibles_and_release_in_the_game_module() {
    let source = r#"
mundo = importe "explorador/mundo"
a = mundo::novo()
b = mundo::novo()
repita 30 vezes { mundo::avance(a, conjunto([:direita]), 1 / 30) }
repita 60 vezes { mundo::avance(b, conjunto([:direita]), 1 / 60) }
diferenca = a:x - b:x
escreva(diferenca > -0.00001 e diferenca < 0.00001)
x = a:x
repita 10 vezes { mundo::avance(a, conjunto(), 0.1) }
escreva(a:x == x, a:quadro == 0, nao a:andando)
repita 30 vezes { mundo::avance(a, conjunto([:direita]), 0.1) }
escreva(a:x <= 118, a:colidiu, nao mundo::colide(a, a:x, a:y))
x = a:x
y = a:y
mundo::avance(a, conjunto([:direita, :baixo]), 0.1)
escreva(a:x == x, a:y > y)
a:x = 100
a:y = 32
mundo::avance(a, conjunto([:direita, :shift]), 10)
escreva(a:x <= 118, nao mundo::colide(a, a:x, a:y))
a:y = 112
repita 8 vezes { mundo::avance(a, conjunto([:direita]), 0.1) }
escreva(a:x > 144, a:camera_x > 0)
mundo::reinicie(a)
mundo::reinicie(b)
mundo::avance(a, conjunto([:direita]), 0.1)
mundo::avance(b, conjunto([:direita, :baixo]), 0.1)
ca = (a:x - 32) * (a:x - 32) + (a:y - 32) * (a:y - 32)
cb = (b:x - 32) * (b:x - 32) + (b:y - 32) * (b:y - 32)
escreva(ca - cb > -0.00001 e ca - cb < 0.00001)
a:x = 83
a:y = 67
mundo::avance(a, conjunto(), 0)
escreva(a:coletados)
mundo::avance(a, conjunto(), 0)
escreva(a:coletados)
mundo::reinicie(a)
escreva(a:coletados, a:x, a:y)
"#;
    for backend in backends() {
        let output = evaluate(source, backend, None);
        assert_eq!(
            output,
            "verdadeiro\nverdadeiro verdadeiro verdadeiro\nverdadeiro verdadeiro verdadeiro\nverdadeiro verdadeiro\nverdadeiro verdadeiro\nverdadeiro verdadeiro\nverdadeiro\n1\n1\n0 32 32\n",
            "{backend:?}"
        );
    }
}

#[test]
fn game_walk_cycle_is_visible_and_resets_when_the_key_is_released() {
    let source = r#"
mundo = importe "explorador/mundo"
s = mundo::novo()
mundo::avance(s, conjunto([:direita]), 0.125)
quadro_anterior = s:quadro
mundo::avance(s, conjunto([:direita]), 0.125)
escreva(quadro_anterior != s:quadro, s:andando)
mundo::avance(s, conjunto(), 0.125)
escreva(s:quadro, s:andando)
"#;
    for backend in backends() {
        assert_eq!(
            evaluate(source, backend, None),
            "verdadeiro verdadeiro\n0 falso\n"
        );
    }
}

struct SlowDisplay {
    actions: VecDeque<Vec<EventoGraf>>,
    input: EstadoEntrada,
    pending: QuadrosPendentes<TelaGrafica>,
    observations: Vec<(i32, i32, usize)>,
    snapshots: usize,
}
impl GrafHost for SlowDisplay {
    fn mostre(&mut self, tela: TelaGrafica) -> Result<(), String> {
        if let Some((x, y, sprite)) = tela.ops.iter().rev().find_map(|op| {
            if let OpGraf::Imagem { x, y, figura, .. } = op {
                Some((*x, *y, figura.px.as_ptr() as usize))
            } else {
                None
            }
        }) {
            self.observations.push((x, y, sprite));
        }
        self.pending.publique(tela.id, tela);
        Ok(())
    }
    fn evento(&mut self, _tela: &TelaGrafica) -> Result<CapturaEvento, String> {
        self.snapshots += 1;
        for event in self
            .actions
            .pop_front()
            .unwrap_or_else(|| vec![EventoGraf::sair()])
        {
            self.input.registre(event);
        }
        Ok(self.input.capture())
    }
}

#[test]
fn complete_playable_example_preserves_release_reset_close_and_last_frame_for_a_slow_consumer() {
    for backend in backends() {
        let mut host = SlowDisplay {
            actions: VecDeque::from([
                vec![EventoGraf::tecla("direita")],
                vec![],
                vec![],
                vec![EventoGraf::soltura("direita")],
                vec![],
                vec![EventoGraf::tecla("direita"), EventoGraf::tecla("baixo")],
                vec![EventoGraf::soltura("direita"), EventoGraf::soltura("baixo")],
                // A complete tap in one capture still resets the game.
                vec![EventoGraf::tecla("r"), EventoGraf::soltura("r")],
                vec![EventoGraf::sair()],
            ]),
            input: EstadoEntrada::default(),
            pending: QuadrosPendentes::default(),
            observations: vec![],
            snapshots: 0,
        };
        assert!(
            evaluate(
                include_str!("../../../../exemplos/jogos/explorador.lep"),
                backend,
                Some(&mut host)
            )
            .is_empty()
        );
        assert_eq!(host.snapshots, 9);
        let observations = &host.observations[1..]; // Skip the preloaded initial scene.
        assert_eq!(observations.len(), 8); // No drawing after close.
        assert_eq!(
            (observations[2].0, observations[2].1),
            (observations[3].0, observations[3].1)
        );
        assert_eq!(observations[3], observations[4]); // Same idle sprite and position.
        assert_ne!(observations[2].2, observations[3].2); // Walking -> idle.
        assert_eq!(
            (observations[6].0, observations[6].1),
            (observations[5].0, observations[5].1)
        );
        assert_eq!((observations[7].0, observations[7].1), (37, 34)); // Reset to spawn.
        let frames = host.pending.retire();
        assert_eq!(frames.len(), 1); // All previous complete frames were replaced.
        let frame = &frames[0];
        assert_eq!((frame.largura, frame.altura), (240, 160));
        let pixels = frame.rgb32();
        assert_eq!(pixels[144 * 240], 0); // HUD must cover the background image.
        assert!(pixels[148 * 240..156 * 240].contains(&0x00FFFFFF)); // HUD text visible.
        assert!(frame.png_bytes().unwrap().len() > 100);
    }
}

#[test]
fn automatic_route_hits_a_wall_stops_and_crosses_its_opening_at_30_and_60_hz() {
    let source = r#"
mundo = importe "explorador/mundo"
roteiro = importe "explorador/roteiro"
para fps em [30, 60] {
    s = mundo::novo()
    t = roteiro::novo()
    estado_valido = verdadeiro
    repita 700 vezes {
        teclas = roteiro::teclas(t, s, 1 / fps)
        mundo::avance(s, teclas, 1 / fps)
        se mundo::colide(s, s:x, s:y) { estado_valido = falso }
        se roteiro::concluido(t) { pare }
    }
    escreva(roteiro::resultado(t), estado_valido, s:quadro == 0)
}
"#;
    for backend in backends() {
        assert_eq!(
            evaluate(source, backend, None),
            "verdadeiro verdadeiro verdadeiro\nverdadeiro verdadeiro verdadeiro\n"
        );
    }
}

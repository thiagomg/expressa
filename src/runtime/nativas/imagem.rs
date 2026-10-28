use crate::lexer::Span;
use crate::runtime::error::EvalError;
use crate::runtime::eval::Vm;
use crate::runtime::imagem::{self, Imagem};
use crate::runtime::value::Value;

impl Vm<'_, '_> {
    pub(crate) fn bi_img_carregue(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let caminho = self.expect_texto(&args[0], span)?;
        let path = self.resolve_data_path(&caminho, span)?;
        let img = imagem::leia_png(&path).map_err(|e| {
            self.err(
                format!("não foi possível ler '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(Value::Imagem(img))
    }

    pub(crate) fn bi_img_salve(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let img = expect_imagem(&args[0], span, self)?;
        let nome = self.expect_texto(&args[1], span)?;
        if !nome.to_ascii_lowercase().ends_with(".png") {
            return Err(self.err("salve espera um arquivo .png", span));
        }
        let path = self.resolve_data_path(&nome, span)?;
        self.ensure_parent_dir(&path, span)?;
        imagem::salve_png(img, &path).map_err(|e| {
            self.err(
                format!("não foi possível salvar '{}': {e}", path.display()),
                span,
            )
        })?;
        Ok(args[0].clone())
    }

    pub(crate) fn bi_img_rotacione(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let img = expect_imagem(&args[0], span, self)?;
        let graus = self.expect_f64(&args[1], span)?;
        let out = imagem::rotacione(img, graus).map_err(|e| self.err(e, span))?;
        Ok(Value::Imagem(out))
    }

    pub(crate) fn bi_img_escala(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        if args.len() != 2 && args.len() != 3 {
            return Err(self.err(
                format!("esperado 2 ou 3 argumento(s), recebeu {}", args.len()),
                span,
            ));
        }
        let img = expect_imagem(&args[0], span, self)?;
        let fx = self.expect_f64(&args[1], span)?;
        let fy = if args.len() == 3 {
            self.expect_f64(&args[2], span)?
        } else {
            fx
        };
        let out = imagem::escala(img, fx, fy).map_err(|e| self.err(e, span))?;
        Ok(Value::Imagem(out))
    }

    pub(crate) fn bi_img_largura(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let img = expect_imagem(&args[0], span, self)?;
        Ok(Value::Numero((img.largura as f64).into()))
    }

    pub(crate) fn bi_img_altura(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let img = expect_imagem(&args[0], span, self)?;
        Ok(Value::Numero((img.altura as f64).into()))
    }
}

pub(crate) fn expect_imagem<'a>(
    v: &'a Value,
    span: Span,
    vm: &Vm<'_, '_>,
) -> Result<&'a Imagem, EvalError> {
    match v {
        Value::Imagem(img) => Ok(img),
        other => Err(vm.err(
            format!("esperada imagem, encontrado {}", other.type_name()),
            span,
        )),
    }
}

#[cfg(test)]
#[allow(clippy::identity_op)] // `1 * 4`: pixel 1, 4 bytes each
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::runtime::eval::run_to_string;

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("expressa-img-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_topo_vermelho(dir: &std::path::Path) -> PathBuf {
        let mut raw = vec![0u8; 3 * 3 * 4];
        raw[1 * 4] = 255;
        raw[1 * 4 + 3] = 255;
        let mut bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut bytes, 3, 3);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(&raw).unwrap();
        }
        let path = dir.join("sprite.png");
        fs::write(&path, bytes).unwrap();
        path
    }

    fn run_in(dir: &std::path::Path, src: &str) -> String {
        let file = dir.join("teste.lep");
        let src = format!("importe \"imagem\"\n{src}");
        run_to_string(&src, file.to_str().unwrap())
            .unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err_in(dir: &std::path::Path, src: &str) -> String {
        let file = dir.join("teste.lep");
        let src = format!("importe \"imagem\"\n{src}");
        match run_to_string(&src, file.to_str().unwrap()) {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    #[test]
    fn carregue_tipo_e_tamanho() {
        let dir = temp_dir();
        write_topo_vermelho(&dir);
        let out = run_in(
            &dir,
            r#"
img = carregue("sprite.png")
escreva(tipo(img))
escreva(largura(img))
escreva(altura(img))
"#,
        );
        assert_eq!(out, "imagem\n3\n3\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn salve_e_carregue_de_novo() {
        let dir = temp_dir();
        write_topo_vermelho(&dir);
        let out = run_in(
            &dir,
            r#"
img = carregue("sprite.png")
img.salve("copia.png")
b = carregue("copia.png")
escreva(b == img)
escreva(b.largura())
"#,
        );
        assert_eq!(out, "verdadeiro\n3\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotacione_noventa_e_escala() {
        let dir = temp_dir();
        write_topo_vermelho(&dir);
        let out = run_in(
            &dir,
            r#"
img = carregue("sprite.png")
r = rotacione(img, 90)
d = escala(img, 2)
escreva(r.largura())
escreva(r.altura())
escreva(d.largura())
escreva(d.altura())
"#,
        );
        assert_eq!(out, "3\n3\n6\n6\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn escala_zero_e_erro() {
        let dir = temp_dir();
        write_topo_vermelho(&dir);
        let err = run_err_in(
            &dir,
            r#"
img = carregue("sprite.png")
escala(img, 0)
"#,
        );
        assert!(err.contains("maior que 0"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }
}

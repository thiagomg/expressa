//! Date and time (`importe "crono"`): `agora`, `data`, `hoje`, `adicione`,
//! `dias_entre`, `eh_bissexto`, `formate_data`.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::lexer::Span;

use super::super::error::EvalError;
use super::super::eval::Vm;
use super::super::numero::Numero;
use super::super::value::{MapKey, Value};

const CAMPOS: [&str; 8] = [
    "ano",
    "mes",
    "dia",
    "hora",
    "minuto",
    "segundo",
    "dia_da_semana",
    "epoch",
];

struct Data {
    ano: i64,
    mes: i64,
    dia: i64,
    hora: i64,
    minuto: i64,
    segundo: i64,
    /// 1 = domingo … 7 = sábado.
    dia_da_semana: i64,
    epoch: i64,
}

impl Vm<'_, '_> {
    pub(crate) fn bi_agora(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        Ok(mapa_de_data(&data_local(epoch_agora())))
    }

    pub(crate) fn bi_hoje(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 0, span)?;
        let n = data_local(epoch_agora());
        self.civil_para_mapa(n.ano, n.mes, n.dia, 0, 0, 0, span)
    }

    pub(crate) fn bi_data(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        let (ano, mes, dia, hora, minuto, segundo) = match args.len() {
            3 => (
                self.expect_int(&args[0], span)?,
                self.expect_int(&args[1], span)?,
                self.expect_int(&args[2], span)?,
                0,
                0,
                0,
            ),
            6 => (
                self.expect_int(&args[0], span)?,
                self.expect_int(&args[1], span)?,
                self.expect_int(&args[2], span)?,
                self.expect_int(&args[3], span)?,
                self.expect_int(&args[4], span)?,
                self.expect_int(&args[5], span)?,
            ),
            n => {
                return Err(self.err(format!("data espera 3 ou 6 argumentos, recebeu {n}"), span));
            }
        };
        self.civil_para_mapa(ano, mes, dia, hora, minuto, segundo, span)
    }

    pub(crate) fn bi_crono_adicione(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 3, span)?;
        let d = self.ler_data(&args[0], span)?;
        let n = self.expect_int(&args[1], span)?;
        let unidade = unidade(&args[2]).ok_or_else(|| {
            self.err(
                "adicione espera a unidade :dias, :meses, :anos, :horas, :minutos ou :segundos",
                span,
            )
        })?;
        let nova = match unidade {
            "dias" => {
                let (ano, mes, dia) = civil_de_dias(dias_desde_civil(d.ano, d.mes, d.dia) + n);
                self.montar(ano, mes, dia, d.hora, d.minuto, d.segundo, span)?
            }
            "meses" => {
                let (ano, mes) = soma_meses(d.ano, d.mes, n);
                let dia = d.dia.min(dias_no_mes(ano, mes));
                self.montar(ano, mes, dia, d.hora, d.minuto, d.segundo, span)?
            }
            "anos" => {
                let ano = d
                    .ano
                    .checked_add(n)
                    .ok_or_else(|| self.err("adicione: ano fora do intervalo", span))?;
                let dia = d.dia.min(dias_no_mes(ano, d.mes));
                self.montar(ano, d.mes, dia, d.hora, d.minuto, d.segundo, span)?
            }
            "horas" => data_local(add_epoch(d.epoch, n, 3600, span, self)?),
            "minutos" => data_local(add_epoch(d.epoch, n, 60, span, self)?),
            "segundos" => data_local(add_epoch(d.epoch, n, 1, span, self)?),
            _ => {
                return Err(self.err(
                    "adicione espera a unidade :dias, :meses, :anos, :horas, :minutos ou :segundos",
                    span,
                ));
            }
        };
        Ok(mapa_de_data(&nova))
    }

    pub(crate) fn bi_dias_entre(&mut self, args: &[Value], span: Span) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let a = self.ler_data(&args[0], span)?;
        let b = self.ler_data(&args[1], span)?;
        let n = dias_desde_civil(b.ano, b.mes, b.dia) - dias_desde_civil(a.ano, a.mes, a.dia);
        Ok(Value::Numero(Numero::from_i64(n)))
    }

    pub(crate) fn bi_eh_bissexto(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 1, span)?;
        let ano = match &args[0] {
            Value::Mapa(_) => self.ler_data(&args[0], span)?.ano,
            _ => self.expect_int(&args[0], span)?,
        };
        Ok(Value::Bool(eh_bissexto(ano)))
    }

    /// `formate_data(data, modelo)`: `%Y %m %d %H %M %S %%`.
    pub(crate) fn bi_formate_data(
        &mut self,
        args: &[Value],
        span: Span,
    ) -> Result<Value, EvalError> {
        self.expect_arity(args, 2, span)?;
        let d = match &args[0] {
            Value::Numero(_) => data_local(self.expect_int(&args[0], span)?),
            Value::Mapa(m) => {
                let mut campos = [0i64; 6];
                for (i, nome) in CAMPOS[..6].iter().enumerate() {
                    let v = campo(m, nome).ok_or_else(|| {
                        self.err(format!("formate_data: falta :{nome} na data"), span)
                    })?;
                    campos[i] = self.expect_int(&v, span)?;
                }
                let [ano, mes, dia, hora, minuto, segundo] = campos;
                Data {
                    ano,
                    mes,
                    dia,
                    hora,
                    minuto,
                    segundo,
                    dia_da_semana: 0,
                    epoch: 0,
                }
            }
            other => {
                return Err(self.err(
                    format!(
                        "formate_data espera o mapa de agora() ou um número, encontrado {}",
                        other.type_name()
                    ),
                    span,
                ));
            }
        };
        let modelo = self.expect_texto(&args[1], span)?;
        let mut out = String::new();
        let mut chars = modelo.chars();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('Y') => out.push_str(&format!("{:04}", d.ano)),
                Some('m') => out.push_str(&format!("{:02}", d.mes)),
                Some('d') => out.push_str(&format!("{:02}", d.dia)),
                Some('H') => out.push_str(&format!("{:02}", d.hora)),
                Some('M') => out.push_str(&format!("{:02}", d.minuto)),
                Some('S') => out.push_str(&format!("{:02}", d.segundo)),
                Some('%') => out.push('%'),
                outro => {
                    return Err(self.err(
                        format!(
                            "formate_data: %{} não existe (use %Y %m %d %H %M %S ou %%)",
                            outro.map(String::from).unwrap_or_default()
                        ),
                        span,
                    ));
                }
            }
        }
        Ok(Value::Texto(out))
    }

    fn civil_para_mapa(
        &self,
        ano: i64,
        mes: i64,
        dia: i64,
        hora: i64,
        minuto: i64,
        segundo: i64,
        span: Span,
    ) -> Result<Value, EvalError> {
        Ok(mapa_de_data(
            &self.montar(ano, mes, dia, hora, minuto, segundo, span)?,
        ))
    }

    fn montar(
        &self,
        ano: i64,
        mes: i64,
        dia: i64,
        hora: i64,
        minuto: i64,
        segundo: i64,
        span: Span,
    ) -> Result<Data, EvalError> {
        valida(ano, mes, dia, hora, minuto, segundo).map_err(|m| self.err(m, span))?;
        let epoch = epoch_de(ano, mes, dia, hora, minuto, segundo)
            .ok_or_else(|| self.err("data fora do intervalo do relógio", span))?;
        let mut d = data_local(epoch);
        d.ano = ano;
        d.mes = mes;
        d.dia = dia;
        d.hora = hora;
        d.minuto = minuto;
        d.segundo = segundo;
        d.dia_da_semana = dia_semana_civil(ano, mes, dia);
        d.epoch = epoch;
        Ok(d)
    }

    fn ler_data(&self, v: &Value, span: Span) -> Result<Data, EvalError> {
        match v {
            Value::Numero(_) => Ok(data_local(self.expect_int(v, span)?)),
            Value::Mapa(m) => {
                let mut campos = [0i64; 6];
                for (i, nome) in CAMPOS[..6].iter().enumerate() {
                    let val = campo(m, nome)
                        .ok_or_else(|| self.err(format!("data: falta :{nome} no mapa"), span))?;
                    campos[i] = self.expect_int(&val, span)?;
                }
                let [ano, mes, dia, hora, minuto, segundo] = campos;
                self.montar(ano, mes, dia, hora, minuto, segundo, span)
            }
            other => Err(self.err(
                format!(
                    "espera o mapa de agora() ou um número (:epoch), encontrado {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }
}

pub(crate) fn eh_data_ou_epoch(v: &Value) -> bool {
    match v {
        Value::Numero(_) => true,
        Value::Mapa(m) => campo(m, "ano").is_some(),
        _ => false,
    }
}

fn campo(m: &super::super::value::MapaRc, nome: &str) -> Option<Value> {
    m.borrow()
        .iter()
        .find(|(k, _)| matches!(k, MapKey::Texto(s) if s == nome))
        .map(|(_, v)| v.clone())
}

fn unidade(v: &Value) -> Option<&str> {
    match v {
        Value::Texto(s) => Some(s.as_str()),
        _ => None,
    }
}

fn mapa_de_data(d: &Data) -> Value {
    let vals = [
        d.ano,
        d.mes,
        d.dia,
        d.hora,
        d.minuto,
        d.segundo,
        d.dia_da_semana,
        d.epoch,
    ];
    Value::mapa(
        CAMPOS
            .iter()
            .zip(vals)
            .map(|(k, v)| {
                (
                    MapKey::Texto((*k).to_string()),
                    Value::Numero(Numero::from_i64(v)),
                )
            })
            .collect(),
    )
}

fn epoch_agora() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn add_epoch(epoch: i64, n: i64, fator: i64, span: Span, vm: &Vm) -> Result<i64, EvalError> {
    n.checked_mul(fator)
        .and_then(|d| epoch.checked_add(d))
        .ok_or_else(|| vm.err("adicione: instante fora do intervalo", span))
}

fn eh_bissexto(ano: i64) -> bool {
    ano % 4 == 0 && (ano % 100 != 0 || ano % 400 == 0)
}

fn dias_no_mes(ano: i64, mes: i64) -> i64 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if eh_bissexto(ano) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn valida(
    ano: i64,
    mes: i64,
    dia: i64,
    hora: i64,
    minuto: i64,
    segundo: i64,
) -> Result<(), String> {
    if !(1..=12).contains(&mes) {
        return Err(format!("mês inválido: {mes}"));
    }
    let max = dias_no_mes(ano, mes);
    if dia < 1 || dia > max {
        return Err(format!("dia inválido: {dia}"));
    }
    if !(0..=23).contains(&hora) {
        return Err(format!("hora inválida: {hora}"));
    }
    if !(0..=59).contains(&minuto) {
        return Err(format!("minuto inválido: {minuto}"));
    }
    if !(0..=59).contains(&segundo) {
        return Err(format!("segundo inválido: {segundo}"));
    }
    Ok(())
}

fn soma_meses(ano: i64, mes: i64, n: i64) -> (i64, i64) {
    let idx = (ano * 12 + (mes - 1)).saturating_add(n);
    let ano = idx.div_euclid(12);
    let mes = idx.rem_euclid(12) + 1;
    (ano, mes)
}

fn dias_desde_civil(mut y: i64, m: i64, d: i64) -> i64 {
    y -= i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_de_dias(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + i64::from(m <= 2);
    (y, m, d)
}

fn dia_semana_civil(ano: i64, mes: i64, dia: i64) -> i64 {
    (dias_desde_civil(ano, mes, dia) + 4).rem_euclid(7) + 1
}

#[cfg(unix)]
fn epoch_de(ano: i64, mes: i64, dia: i64, hora: i64, minuto: i64, segundo: i64) -> Option<i64> {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    tm.tm_year = i32::try_from(ano - 1900).ok()?;
    tm.tm_mon = i32::try_from(mes - 1).ok()?;
    tm.tm_mday = i32::try_from(dia).ok()?;
    tm.tm_hour = i32::try_from(hora).ok()?;
    tm.tm_min = i32::try_from(minuto).ok()?;
    tm.tm_sec = i32::try_from(segundo).ok()?;
    tm.tm_isdst = -1;
    let t = unsafe { libc::mktime(&mut tm) };
    if t == -1 {
        return None;
    }
    Some(t)
}

#[cfg(not(unix))]
fn epoch_de(ano: i64, mes: i64, dia: i64, hora: i64, minuto: i64, segundo: i64) -> Option<i64> {
    dias_desde_civil(ano, mes, dia)
        .checked_mul(86_400)
        .and_then(|d| d.checked_add(hora * 3600 + minuto * 60 + segundo))
}

#[cfg(unix)]
fn data_local(epoch: i64) -> Data {
    let t = epoch as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
        return data_utc(epoch);
    }
    Data {
        ano: i64::from(tm.tm_year) + 1900,
        mes: i64::from(tm.tm_mon) + 1,
        dia: i64::from(tm.tm_mday),
        hora: i64::from(tm.tm_hour),
        minuto: i64::from(tm.tm_min),
        segundo: i64::from(tm.tm_sec),
        dia_da_semana: i64::from(tm.tm_wday) + 1,
        epoch,
    }
}

#[cfg(not(unix))]
fn data_local(epoch: i64) -> Data {
    data_utc(epoch)
}

fn data_utc(epoch: i64) -> Data {
    let dias = epoch.div_euclid(86_400);
    let resto = epoch.rem_euclid(86_400);
    let (ano, mes, dia) = civil_de_dias(dias);
    Data {
        ano,
        mes,
        dia,
        hora: resto / 3600,
        minuto: resto % 3600 / 60,
        segundo: resto % 60,
        dia_da_semana: (dias + 4).rem_euclid(7) + 1,
        epoch,
    }
}

#[cfg(test)]
mod tests {
    use super::{civil_de_dias, data_utc, dias_desde_civil, eh_bissexto};
    use crate::runtime::run_to_string;

    fn run(src: &str) -> String {
        run_to_string(src, "teste.lep").unwrap_or_else(|e| panic!("run failed: {e}\n{src}"))
    }

    fn run_err(src: &str) -> String {
        match run_to_string(src, "teste.lep") {
            Err(e) => e.message,
            Ok(out) => panic!("expected error, got {out:?}\n{src}"),
        }
    }

    const IMP: &str = "importe \"crono\"\n";

    #[test]
    fn agora_e_formate_data() {
        assert!(run_err("agora()").contains("não definida"));
        assert_eq!(
            run(&format!(
                "{IMP}d = agora()\nescreva(d:mes >= 1 e d:mes <= 12, d:dia_da_semana <= 7, \
                 formate_data(d, \"%Y\") == formate_data(d:epoch, \"%Y\"))"
            )),
            "verdadeiro verdadeiro verdadeiro\n"
        );
        assert_eq!(
            run(&format!(
                "{IMP}d = mapa([:ano -> 2024, :mes -> 3, :dia -> 9, :hora -> 7, :minuto -> 5, \
                 :segundo -> 0])\nescreva(formate_data(d, \"%Y-%m-%d %H:%M:%S 100%%\"))"
            )),
            "2024-03-09 07:05:00 100%\n"
        );
        assert!(run_err(&format!("{IMP}formate_data(agora(), \"%x\")")).contains("%x não existe"));
        assert!(
            run_err(&format!("{IMP}formate_data(mapa([:ano -> 1]), \"%Y\")"))
                .contains("falta :mes")
        );
    }

    #[test]
    fn construtor_hoje_e_bissexto() {
        assert_eq!(
            run(&format!(
                "{IMP}d = data(2024, 2, 29)\nescreva(formate_data(d, \"%Y-%m-%d\"), d:dia_da_semana)"
            )),
            "2024-02-29 5\n"
        );
        assert!(run_err(&format!("{IMP}data(2023, 2, 29)")).contains("dia inválido"));
        assert_eq!(
            run(&format!(
                "{IMP}h = hoje()\nescreva(h:hora, h:minuto, h:segundo)"
            )),
            "0 0 0\n"
        );
        assert_eq!(
            run(&format!(
                "{IMP}escreva(eh_bissexto(2024), é_bissexto(2023), eh_bissexto(data(2000, 1, 1)))"
            )),
            "verdadeiro falso verdadeiro\n"
        );
        assert!(!eh_bissexto(1900));
        assert!(eh_bissexto(2000));
    }

    #[test]
    fn adicione_e_dias_entre() {
        assert_eq!(
            run(&format!(
                "{IMP}d = data(2024, 1, 31).adicione(1, :meses)\n\
                 escreva(formate_data(d, \"%Y-%m-%d\"))"
            )),
            "2024-02-29\n"
        );
        assert_eq!(
            run(&format!(
                "{IMP}d = data(2023, 1, 31).adicione(1, :meses)\n\
                 escreva(formate_data(d, \"%Y-%m-%d\"))"
            )),
            "2023-02-28\n"
        );
        assert_eq!(
            run(&format!(
                "{IMP}a = data(2024, 1, 1)\nb = adicione(a, 7, :dias)\n\
                 escreva(formate_data(b, \"%Y-%m-%d\"), dias_entre(a, b), dias_entre(b, a))"
            )),
            "2024-01-08 7 -7\n"
        );
        assert_eq!(
            run(&format!(
                "{IMP}d = data(2024, 3, 10, 10, 0, 0).adicione(-2, :horas)\n\
                 escreva(formate_data(d, \"%H:%M\"))"
            )),
            "08:00\n"
        );
        assert!(
            run_err(&format!("{IMP}adicione(data(2024, 1, 1), 1, :semanas)")).contains("unidade")
        );
    }

    #[test]
    fn datas_utc() {
        let d = data_utc(0);
        assert_eq!(
            (d.ano, d.mes, d.dia, d.hora, d.dia_da_semana),
            (1970, 1, 1, 0, 5)
        );
        let d = data_utc(1_709_212_455);
        assert_eq!(
            (
                d.ano,
                d.mes,
                d.dia,
                d.hora,
                d.minuto,
                d.segundo,
                d.dia_da_semana
            ),
            (2024, 2, 29, 13, 14, 15, 5)
        );
        let d = data_utc(-1);
        assert_eq!((d.ano, d.mes, d.dia, d.hora), (1969, 12, 31, 23));
        assert_eq!(dias_desde_civil(1970, 1, 1), 0);
        assert_eq!(civil_de_dias(0), (1970, 1, 1));
    }
}

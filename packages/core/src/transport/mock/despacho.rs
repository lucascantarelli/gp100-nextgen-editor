//! o DESPACHO do envelope GP-100: pedido -> efeito de estado + resposta (D1/D3/D5).

use std::time::Duration;

use super::respostas::{build_from_example, build_with_fill, example_var, push_template};
use super::{DeviceTransport, MockDevice, MockFault, TransportError, WireKind};
use crate::golden::decode_envelope;
use crate::golden::GoldenFile;
use crate::ProtocolError;

impl MockDevice {
    /// Processa UM frame de entrada (já decodificado): efeito de estado +
    /// resposta a enfileirar, se houver. É o coração do despacho (D1/D3/D5).
    fn ingest(&mut self, func: u8, addr: [u8; 4], payload: &[u8]) -> Result<(), TransportError> {
        // ADR-2/rust-practices: sem panic na lib — golden indisponível vira
        // erro tipado na MESMA conversão do resto do despacho (D5).
        let golden = match GoldenFile::embedded() {
            Ok(g) => g,
            Err(e) => {
                self.state.rejected += 1;
                return Err(TransportError::SendFailed { why: e.to_string() });
            }
        };

        // §13.11 set_param: o golden congela os templates POR INSTÂNCIA
        // (9 templates 10xx0002 com consts de knobs específicos), então o
        // SHAPE é validado pelo CODEC (prova: 92 knobs byte a byte em
        // tests/codec_wire.rs). R1: shape ≠ semântica — o codec é a fonte.
        if func == 0x12 && addr[0] == 0x10 && addr[2] == 0x00 && addr[3] == 0x02 {
            return match crate::codec::set_param_parse(payload) {
                Ok((code, ctrl, value)) => {
                    let nib = (code >> 24) as u8;
                    let idx = code & 0x00FF_FFFF;
                    let _known = self.dict.algorithm(nib, idx).is_some();
                    self.state.set_params.insert((nib, ctrl), (code, value));
                    Ok(()) // D4: sem resposta
                }
                Err(e) => {
                    self.state.rejected += 1;
                    Err(TransportError::SendFailed { why: e.to_string() })
                }
            };
        }
        // §13.13 registradores globais (família 000200xx/000102xx): fora do
        // golden (mundo do preset) — a forma é a medida (4B/8B, §13.13.1).
        // O mock guarda o valor num table: read (`11/<addr>` sem payload)
        // responde com o valor; write (`12/<addr>` 4B) atualiza (D4: sem
        // resposta — o re-push de confirmação do Suite não é modelado).
        if addr[0] == 0x00 && (addr[1] == 0x01 || addr[1] == 0x02) {
            if let Some(valor) = self.state.globals.get(&addr) {
                return match (func, payload.len()) {
                    (0x11, 0) => {
                        let mut msg =
                            Vec::with_capacity(crate::SYSEX_HEADER.len() + 6 + valor.len());
                        msg.extend_from_slice(&crate::SYSEX_HEADER);
                        msg.push(0x12);
                        msg.extend_from_slice(&addr);
                        msg.extend_from_slice(valor);
                        msg.push(crate::SYSEX_EOX);
                        self.inbox.entry((0x12, addr)).or_default().push(msg);
                        Ok(())
                    }
                    (0x12, 4) => {
                        self.state.globals.insert(addr, payload.to_vec());
                        Ok(())
                    }
                    _ => {
                        self.state.rejected += 1;
                        Err(TransportError::SendFailed {
                            why: format!(
                                "global {addr:02x?}: forma não medida (func={func:02x} len={})",
                                payload.len()
                            ),
                        })
                    }
                };
            }
        }
        // D5: o frame DEVE casar com um request_pattern do endpoint.
        let Some(tpl_idx) = golden.match_request(func, &addr, payload) else {
            self.state.rejected += 1;
            return Err(TransportError::SendFailed {
                why: format!(
                    "request sem template no golden (D5): func={func:02x} addr={addr:02x?} payload={:.24}",
                    payload.iter().map(|b| format!("{b:02x}")).collect::<String>()
                ),
            });
        };

        // Efeitos de estado dos WRITES (D3: nenhum gera resposta).
        match (func, addr) {
            // seleção de preset (§13.10): payload = [pp BE]
            (0x11, [0x13, 0x01, 0x00, 0x00]) if payload.len() == 2 => {
                self.state.current_pp = u16::from_be_bytes([payload[0], payload[1]]);
            }
            // metadados do save (§13.12): nome (ASCII 12B trunc+pad)
            (0x12, [0x11, 0x00, 0x00, 0x00]) if payload.len() == 20 => {
                let name: String = payload[8..20]
                    .iter()
                    .take_while(|&&b| b != 0)
                    .map(|&b| b as char)
                    .collect();
                if !name.is_empty() {
                    self.state.current_name = name;
                }
            }
            // ppType u16 BE + zeros2
            (0x12, [0x11, 0x00, 0x05, 0x00]) if payload.len() == 4 => {
                self.state.current_pp_type = u16::from_be_bytes([payload[0], payload[1]]);
            }
            // (set_param tratado ANTES do despacho — shape é do codec)
            _ => {}
        }

        // Respostas (D1): o SELECT de preset (write 11/13010000) responde
        // com o meta6 push em `13010001` levando o pp SELECIONADO —
        // pareamento 199↔199 provado no replay S1 (e na sonda 1302); READS
        // func 11 respondem com o push do MESMO endereço (func 12 no fio);
        // transações req 12-func com lado IN respondem pelo template casado.
        // Sonda do banco 02 (T6 do boot §13.10): select CONST `0000` (t14;
        // sem pp) → o meta6 PUSH da sonda (t12 @ `13020001`, const
        // `00000c1c0140` — o device não ecoa pp porque o select não tem pp).
        let reply = if func == 0x11 && addr == [0x13, 0x02, 0x00, 0x00] && payload == [0x00, 0x00] {
            push_template(golden, [0x13, 0x02, 0x00, 0x01])
                .ok()
                .and_then(|t| build_from_example(t, None, None).ok())
        } else if func == 0x11 && addr == [0x13, 0x01, 0x00, 0x00] && payload.len() == 2 {
            let pp = payload[..2].to_vec();
            push_template(golden, [0x13, 0x01, 0x00, 0x01])
                .ok()
                .and_then(|t| {
                    build_with_fill(t, None, |i, count| {
                        if i == 0 && count == 2 {
                            pp.clone()
                        } else {
                            example_var(t, None, i, count)
                        }
                    })
                    .ok()
                })
        } else if func == 0x11 {
            let tpl = push_template(golden, addr).ok();
            let desired = if addr == [0x12, 0x00, 0x10, 0x02] {
                Some(75)
            } else {
                None
            }; // Tabela de IRs (t2/t3): o IN ecoa a PÁGINA pedida no byte[0]
               // (captura S1 rows 78–84: IN `13 0f…` para a leitura `[13]`).
               // Vale para o var inteiro (ACK 1B ou tabela 75B): byte 0 = página.
            let page = payload.first().copied();
            let is_table = addr == [0x12, 0x00, 0x10, 0x02];
            tpl.and_then(|t| {
                build_with_fill(t, desired, |i, count| {
                    if is_table && i == 0 {
                        let mut v = example_var(t, None, 0, count);
                        if let (Some(p), Some(b0)) = (page, v.first_mut()) {
                            *b0 = p;
                        }
                        v
                    } else {
                        example_var(t, None, i, count)
                    }
                })
                .ok()
            })
        } else {
            match (func, addr) {
                // ACK de chunk de IR (§13.7): eco [slot][idx BE] + 01
                (0x12, [0x12, 0x00, 0x10, 0x02]) => {
                    let tpl = &golden.templates()[tpl_idx];
                    let echo = payload[..3].to_vec();
                    build_with_fill(tpl, Some(4), |i, count| {
                        if i == 0 {
                            let mut v = echo.clone();
                            v.resize(count, 0);
                            v
                        } else {
                            example_var(tpl, None, i - 1, count)
                        }
                    })
                    .ok()
                }
                // meta6 (t6, const) / abertura (t7/t13) / 13010005 (t11/
                // t16) / páginas (t8/t15, by-len).
                //
                // Regra do arm: o shape vem SEMPRE do template (mock não
                // inventa forma, D5) e o conteúdo por EVIDÊNCIA:
                //   abertura → o device responde com a PÁGINA 0 (196B) no
                //     endpoint de página — ordem da captura S1 rows 89–93
                //     (open open → pág0 pág0); o golden t7 pareou a abertura
                //     com o meta6 do select (mis-pairing inofensivo até
                //     aqui: o boot só tinha rodado no transporte de replay);
                //   páginas → by-len 196/32 pelo in_addr do template, com
                //     eco do pp do REQUEST e o número da página RESPONDIDA
                //     (= PG+1, §13.10 — evidência S1 rows 93–107);
                //   PG 8 → 4B em `1301/1302 0005` (t11/t16, pp no var0).
                (0x12, [0x13, ..]) => {
                    let tpl = &golden.templates()[tpl_idx];
                    // by-len decide-se pelo ENDEREÇO DA RESPOSTA (IN do
                    // template), não pelo frame recebido: t8 lê em
                    // 13010004 e responde em 13010003 (196B/32B).
                    let in_addr = tpl
                        .in_
                        .as_ref()
                        .and_then(|e| e.parsed().ok())
                        .map(|(_, a)| a);
                    let desired = match in_addr {
                        Some([0x13, 0x01, 0x00, 0x03]) | Some([0x13, 0x02, 0x00, 0x03]) => {
                            Some(196)
                        }
                        _ => None,
                    };
                    // Eco do pp VEM DO REQUEST (payload[0..2]): o scan abre/
                    // pagina o pp pedido — o state.current_pp ainda é o
                    // ANTERIOR durante o scan. Fallback = corrente.
                    let echo_pp = payload
                        .get(0..2)
                        .map(|s| s.to_vec())
                        .unwrap_or_else(|| self.state.current_pp.to_be_bytes().to_vec());
                    let requested_pg = payload.get(3).copied();
                    // A página 8 — resposta ao req PG 7 — mede 32B na
                    // captura real (4B header + 28B de nibbles = 14
                    // decodificados); as outras 8 medem 196B (4B + 192B).
                    // O golden aceita os dois (`lens: [32, 196]` em
                    // `13010003`), então servir 196B em tudo passava
                    // silenciosamente: o shape batia por forma e só o
                    // `decode` das páginas (§13.10) acusava, como 9 páginas
                    // de 196B onde o aparelho manda 8+1.
                    let desired = match requested_pg {
                        Some(7) => Some(32),
                        _ => desired,
                    };
                    let is_page =
                        matches!(addr, [0x13, 0x01, 0x00, 0x04] | [0x13, 0x02, 0x00, 0x04]);
                    // PG 8 (§13.10): a resposta NÃO é página 196B — é 4B em
                    // `1301/1302 0005` (t11/t16, pp no var0; evidência S1
                    // rows 107–109). A by-len não tem sub-padrão 4B.
                    let pg8_in: [u8; 4] = if addr[1] == 0x02 {
                        [0x13, 0x02, 0x00, 0x05]
                    } else {
                        [0x13, 0x01, 0x00, 0x05]
                    };
                    let pg8 = if is_page && payload.get(3) == Some(&0x08) {
                        golden
                            .templates()
                            .iter()
                            .find(|t| {
                                t.template_type == "req"
                                    && t.in_.as_ref().is_some_and(|e| {
                                        e.parsed().map(|(_, a)| a).is_ok_and(|a| a == pg8_in)
                                    })
                            })
                            .and_then(|t11| {
                                build_with_fill(t11, None, |i, count| {
                                    if i == 0 && count == 2 {
                                        echo_pp.clone()
                                    } else {
                                        example_var(t11, None, i, count)
                                    }
                                })
                                .ok()
                            })
                    } else {
                        None
                    };
                    if let Some(msg) = pg8 {
                        Some(msg)
                    } else {
                        let is_open =
                            matches!(addr, [0x13, 0x01, 0x00, 0x02] | [0x13, 0x02, 0x00, 0x02])
                                && payload.len() == 3;
                        if is_open {
                            let want_in: [u8; 4] = if addr[1] == 0x02 {
                                [0x13, 0x02, 0x00, 0x03]
                            } else {
                                [0x13, 0x01, 0x00, 0x03]
                            };
                            // Variante #161: banco 01 (o scan que enche o
                            // cache `self.pages`) com o pp pedido na lista
                            // de slots sem nome → corpo da pg0 zerado. A
                            // sonda 02 não vai ao cache, fica de fora.
                            let sem_nome = addr[1] == 0x01
                                && self
                                    .slots_sem_nome
                                    .iter()
                                    .any(|&pp| echo_pp == pp.to_be_bytes());
                            let tpl_page = golden
                                .templates()
                                .iter()
                                .find(|t| {
                                    t.template_type == "req"
                                        && t.in_.as_ref().is_some_and(|e| {
                                            e.parsed().map(|(_, a)| a).is_ok_and(|a| a == want_in)
                                        })
                                })
                                .ok_or(TransportError::SendFailed {
                                    why: "template da página (t8/t15) ausente no golden".into(),
                                })?;
                            let page = build_with_fill(tpl_page, Some(196), |i, count| {
                                if i == 0 && count == 2 {
                                    echo_pp.clone()
                                } else if i == 1 && count == 1 {
                                    vec![0] // PG 0 da abertura
                                } else if sem_nome {
                                    // Slot sem nome (#161): o corpo vem
                                    // zerado — mesmos bytes de shape, zero
                                    // de conteúdo. `decode` aceita, `nome`
                                    // recusa "nome vazio".
                                    vec![0; count]
                                } else {
                                    // O exemplo congelado do golden É o
                                    // conteúdo real — é o mesmo caminho dos
                                    // reqs de página. O corpo do `open` é a
                                    // pg0, onde mora o NOME do preset (#155);
                                    // zeros aqui deixavam `preset_pages::nome`
                                    // sem o que ler e derrubavam o palco.
                                    example_var(tpl_page, Some(196), i, count)
                                }
                            })
                            .map_err(|e: ProtocolError| {
                                TransportError::SendFailed { why: e.to_string() }
                            })?;
                            Some(page)
                        } else {
                            // Páginas (t8/t15, by-len): eco do request onde
                            // evidente e exemplo da resposta para o resto —
                            // shape sempre correto (by-len casa por forma).
                            //
                            // O layout do segmento `by_len["196"]` é
                            // `[var 2B = pp][const 1B = 00][var 18B = corpo]…`,
                            // e o NÚMERO DA PÁGINA é o byte 0 desse var de 18B
                            // — é o `d[3]` do §13.10, que substituiu o
                            // "contador de versão" do §13.3.
                            //
                            // O número é `PG + 1`, NÃO `PG`: o §13.10 diz
                            // "req pg0..7 → pág1..8" (e a PG 8 responde o
                            // header de 4B com d[3]=9). Ecoar o PG do request
                            // fazia TODAS as 8 páginas responderem 0.
                            //
                            // O estrago disso era invisível e caro: o
                            // `StatePage` é opaco (ninguém lia esse byte), o
                            // dump prints OK, e a referência de
                            // `h1_reference` (gate H1) — que é a SAÍDA do
                            // mock — ficava com drift. Como o H1 compara o
                            // device real contra essa referência, cada
                            // sessão de campo acusaria 8 "divergências de
                            // estado" por dump que não são de estado — e
                            // elas seriam coladas no H1_REPORT como afirmações
                            // sobre a pedaleira.
                            build_with_fill(tpl, desired, |i, count| {
                                if i == 0 && count == 2 {
                                    echo_pp.clone()
                                } else if is_page && i == 1 && count >= 1 {
                                    let mut corpo = example_var(tpl, desired, i, count);
                                    // o exemplo congelado é a evidência do
                                    // corpo; só d[3] (offset 3 do payload) é
                                    // reescrito com a página desta resposta.
                                    corpo[0] = requested_pg.unwrap_or(0).wrapping_add(1);
                                    corpo
                                } else {
                                    example_var(tpl, desired, i, count)
                                }
                            })
                            .map(Some)
                            .map_err(|e: ProtocolError| {
                                TransportError::SendFailed { why: e.to_string() }
                            })?
                        }
                    }
                }
                _ => None,
            }
        };

        if let Some(msg) = reply {
            let in_key = decode_envelope(&msg)
                .map(|(f, a, _)| (f, a))
                .unwrap_or((func, addr));
            self.inbox.entry(in_key).or_default().push(msg);
        }
        Ok(())
    }
}

impl DeviceTransport for MockDevice {
    fn open(&mut self) -> Result<(), TransportError> {
        // Device que CAIU não ressuscita no mock (o re-plug é sessão nova):
        // reabrir o mesmo objeto devolve `DeviceGone`, não `Closed`.
        if self.gone() {
            return Err(self.gone_err());
        }
        self.opened = true;
        Ok(())
    }

    fn close(&mut self) -> Result<(), TransportError> {
        self.opened = false;
        Ok(())
    }

    /// O mock **sempre** aceita escrita (ADR-5): a trava é do transporte
    /// REAL, e o mock é o test double dos replays de knob/save/IR — uma
    /// trava aqui apagaria justamente os ensumos que provam o codec.
    fn send_raw(&mut self, data: &[u8], _kind: WireKind) -> Result<(), TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        // A MORTE precede o parse: um device que já não está no fio não
        // recebe bytes, então o plano de falha é checado ANTES do envelope
        // (o shape do frame não importa para quem sumiu).
        self.sent += 1;
        if self.gone() {
            return Err(self.gone_err());
        }
        // O USB instável precede o parse pelo MESMO motivo: o SO recusou o
        // buffer e o device nunca viu os bytes — SendFailed TRANSITÓRIO,
        // device vivo, e a tentativa seguinte passa (contador saiu do
        // múltiplo). Nenhuma resposta nasce de um envio que não chegou.
        if let Some(MockFault::UsbFlaky { send_every, .. }) = self.fault {
            if send_every > 0 && self.sent.is_multiple_of(send_every) {
                self.transient += 1;
                return Err(TransportError::SendFailed {
                    why: format!(
                        "mock: USB instável — o envio {0} (múltiplo de {send_every}) \
                         falhou de forma transitória (MockFault::UsbFlaky)",
                        self.sent
                    ),
                });
            }
        }
        // Latência de IDA (opt-in): os bytes chegam ao device depois do
        // jitter. Sem `with_latency` não existe um único sleep neste mock.
        if self.latency.is_some() {
            let jitter = self.proximo_jitter();
            std::thread::sleep(jitter);
        }
        // O framing da FAMÍLIA (SnapTone/NAM, §5) vem antes do decode do
        // envelope: `F0` + nibbles + `F7` não tem o cabeçalho de 8B do
        // GP-100, então sem esta volta um bloco de SnapTone viraria
        // SendFailed "cabeçalho inválido" e o upload do §5 — o único
        // caminho de escrita com ACK por bloco que dá para testar sem
        // hardware — ficaria impossível de exercitar.
        if crate::snap_tone::eh_familia(data) {
            return self.ingest_familia(data);
        }
        let (func, addr, payload) =
            decode_envelope(data).map_err(|e| TransportError::SendFailed { why: e.to_string() })?;
        self.ingest(func, addr, payload)
    }

    fn recv_raw(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        if !self.opened {
            return Err(TransportError::Closed);
        }
        if self.gone() {
            return Err(self.gone_err());
        }
        // D1: FIFO global — a mensagem mais antiga de qualquer endpoint.
        let oldest = self
            .inbox
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, _)| *k)
            .min();
        if let Some(k) = oldest {
            // sai da fila ANTES dos sleeps: o `&mut self` do próximo_jitter
            // não pode conviver com o empréstimo do `inbox`.
            let popped = match self.inbox.get_mut(&k) {
                Some(q) if !q.is_empty() => Some(q.remove(0)),
                _ => None,
            };
            if let Some(msg) = popped {
                // Latência de VOLTA (opt-in): a mensagem passa pelo fio
                // antes de chegar ao host.
                if self.latency.is_some() {
                    let jitter = self.proximo_jitter();
                    std::thread::sleep(jitter);
                }
                // USB instável: a n-ésima mensagem que SAÍ da fila é
                // PERDIDA no fio — consumida aqui (não reaparece) e o host
                // vê RecvTimeout com o device saudável. Sem latência
                // optada o retorno é imediato: a prova do drop é a fila
                // encurtar (a seguinte entrega normalmente), não o relógio.
                // O contador é de SAÍDAS, não de entregas — senão o segundo
                // drop prenderia toda a fila restante.
                self.popped += 1;
                if let Some(MockFault::UsbFlaky { drop_every, .. }) = self.fault {
                    if drop_every > 0 && self.popped.is_multiple_of(drop_every) {
                        return Err(TransportError::RecvTimeout {
                            timeout_ms: timeout.as_millis() as u64,
                        });
                    }
                }
                return Ok(msg);
            }
        }
        // Fila vazia: SEM latência o mock é o de sempre — RecvTimeout
        // IMEDIATO (o default que a suíte inteira conhece). COM latência
        // optada ele espelha o `real.rs`: respeita a JANELA da chamada e só
        // então acusa o silêncio. Aqui morria o falso positivo de tempo
        // (janela de 3s resolvida em microssegundos).
        if self.latency.is_some() {
            std::thread::sleep(timeout);
        }
        Err(TransportError::RecvTimeout {
            timeout_ms: timeout.as_millis() as u64,
        })
    }
}

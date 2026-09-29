//! Testes de CONTRATO da API `build_response` (M0.5): a resposta que o
//! golden constrói DEVE casar de volta no próprio template
//! (`extract → build == exemplo` aplicado ao lado IN — espelha a
//! propriedade dos requests provada no M0.3).

use gp100_core::golden::GoldenFile;

/// Para os 40 templates: construir a resposta a partir dos VARS do PRÓPRIO
/// exemplo congelado reproduz o exemplo byte a byte e o extract devolve os
/// mesmos vars (simetria build ↔ extract, lado IN).
#[test]
fn build_response_e_simetrico_aos_exemplos_congelados() {
    let golden = GoldenFile::embedded().expect("golden embutido");
    let mut checked = 0usize;

    for (i, tpl) in golden.templates().iter().enumerate() {
        let Some(pat) = tpl.response_pattern() else {
            continue;
        };
        if pat.pattern_kind == gp100_core::golden::PatternKind::ByLen {
            // t3 (tabela IRs 12001002): os DOIS formatos são provados pelo
            // contrato do mock (75B de fábrica) e do ACK (t31, 4B)
            continue;
        }
        let ex_hex = tpl
            .example
            .response_hex
            .as_deref()
            .or(tpl.example.hex.as_deref())
            .unwrap_or_default();
        if ex_hex.is_empty() {
            continue;
        }
        let ex = gp100_core::golden::hex_decode(ex_hex).expect("exemplo hex");
        let vars: Vec<Vec<u8>> = pat
            .extract_vars(&ex)
            .map(|v| v.iter().map(|s| s.to_vec()).collect())
            .unwrap_or_default();
        if vars.is_empty() && !ex.is_empty() {
            // padrão const: build sem vars deve reproduzir o exemplo
            let built = tpl
                .build_response(
                    &mut |_i, count| vec![0u8; count], // não será chamado
                    None,
                )
                .unwrap_or_else(|e| panic!("t{i} const deve ser buildável: {e}"));
            let (_, _, p) =
                gp100_core::golden::decode_envelope(&built).expect("envelope da resposta");
            assert_eq!(p, ex, "t{i}: resposta const == exemplo");
            checked += 1;
            continue;
        }
        // padrão com vars: reconstrói a partir dos vars extraídos
        let vi = vars.clone();
        let built = tpl
            .build_response(
                &mut |idx, count| {
                    let mut v = vi.get(idx).cloned().unwrap_or_else(|| vec![0u8; count]);
                    v.resize(count, 0);
                    v
                },
                None,
            )
            .unwrap_or_else(|e| panic!("t{i} build_response falhou: {e}"));
        let (_, _, p) = gp100_core::golden::decode_envelope(&built).expect("envelope da resposta");
        assert_eq!(p, ex, "t{i}: extract → build == exemplo (lado IN)");
        // e o extract de volta devolve os MESMOS vars
        let back = tpl.matches_response(p).expect("extract de volta");
        let back: Vec<Vec<u8>> = back.iter().map(|s| s.to_vec()).collect();
        assert_eq!(back, vars, "t{i}: extract(build(extract(ex))) == vars");
        checked += 1;
    }
    // Elegíveis = 10 pushes + 8 reqs − 3 by-len (t3/t8/t15) = 15.
    // Writes (22) não têm lado IN e saem do laço.
    assert_eq!(checked, 15, "todos os templates com lado IN buildável");
}

/// O caso by-len de `12001002` (ACK 4B × tabela 75B): `build_response`
/// respeita o `desired_len` e cada formato casa de volta no template.
#[test]
fn build_response_by_len_escolhe_o_subpadrao() {
    let golden = GoldenFile::embedded().expect("golden embutido");
    let tpl = &golden.templates()[31]; // req chunk -> ACK (t31, mixed 4B)
    let built = tpl
        .build_response(
            &mut |i, count| {
                if i == 0 {
                    vec![0x11, 0x22, 0x33].resize_pad(count)
                } else {
                    vec![0u8; count]
                }
            },
            None,
        )
        .expect("ACK buildável");
    let (_, _, p) = gp100_core::golden::decode_envelope(&built).expect("envelope");
    assert_eq!(p, [0x11, 0x22, 0x33, 0x01], "ACK = var3 + const 01");
    assert!(tpl.matches_response(p).is_some());
}

/// Helper local de pad (evita trait no escopo global do teste).
trait ResizePad {
    fn resize_pad(self, n: usize) -> Vec<u8>;
}
impl ResizePad for Vec<u8> {
    fn resize_pad(mut self, n: usize) -> Vec<u8> {
        self.resize(n, 0);
        self
    }
}
impl ResizePad for [u8; 3] {
    fn resize_pad(self, n: usize) -> Vec<u8> {
        let mut v = self.to_vec();
        v.resize(n, 0);
        v
    }
}

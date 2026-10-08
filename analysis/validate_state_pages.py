#!/usr/bin/env python3
"""validate_state_pages.py — a trava do artefato de layout das páginas 13xx (#155).

O artefato e embutido no binario por `preset_pages::slots()`. Um artefato que
"quase" casa e pior que nenhum: o palco desenha e o usuario acredita. Este
gate recusa o artefato se:

  * o encoding nao for `nibble`;
  * o metodo nao for o modelo deterministico (offset CALCULADO, nao procurado
    por votacao — a votacao deu share_min baixo e e o metodo fraco);
  * as provas cairem abaixo da barra;
  * o span de params nao cobrir exatamente os 135 slots;
  * o layout nao tiver os campos que `preset_pages::slots()` consome.

Roda dentro do `pytest` (regime do `param_ranges.py`) e como script.
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ARTEFATO = os.path.join(HERE, "state_pages_offsets.json")

# Mesma barra do gerador — se mudar uma, muda a outra (o teste acusa).
TAXA_MIN = 0.95
PARAMS_N = 135
PARAMS_POR_SLOT = 15
CADEIA_N = 9
CODE_N = 9
STATE_N = 9
NOME_OFF, NOME_LEN = 2, 12
ESPERADO_METHOD = "modelo-deterministico (offset calculado pela estrutura)"


def carregar():
    if not os.path.exists(ARTEFATO):
        return None, ["state_pages_offsets.json ausente — rode map_state_pages.py"]
    try:
        with open(ARTEFATO, encoding="utf8") as fh:
            return json.load(fh), []
    except Exception as e:  # JSON quebrado tambem e recusa
        return None, [f"JSON invalido: {e}"]


def validar(a):
    erros = []
    if a.get("encoding") != "nibble":
        erros.append(f"encoding={a.get('encoding')!r} (esperado 'nibble')")
    if a.get("method") != ESPERADO_METHOD:
        erros.append(f"method={a.get('method')!r} (esperado o modelo deterministico)")

    for campo in ("taxa_params", "taxa_state"):
        v = a.get(campo)
        if not isinstance(v, (int, float)):
            erros.append(f"{campo} ausente")
        elif v < TAXA_MIN:
            erros.append(f"{campo}={v} < {TAXA_MIN}")

    lay = a.get("layout")
    if not isinstance(lay, dict):
        erros.append("layout ausente")
        return erros

    for campo in ("pp", "nome", "cadeia", "effectCode", "effectState", "params"):
        if campo not in lay:
            erros.append(f"layout.{campo} ausente")

    nome = lay.get("nome", {})
    if nome.get("offset") != NOME_OFF or nome.get("length") != NOME_LEN:
        erros.append(
            f"nome offset={nome.get('offset')} length={nome.get('length')} "
            f"(esperado {NOME_OFF}/{NOME_LEN})"
        )

    cadeia = lay.get("cadeia", {})
    if cadeia.get("count") != CADEIA_N:
        erros.append(f"cadeia count={cadeia.get('count')} (esperado {CADEIA_N})")

    code = lay.get("effectCode", {})
    if code.get("count") != CODE_N or code.get("width") != 4:
        erros.append(f"effectCode count={code.get('count')} width={code.get('width')}")

    state = lay.get("effectState", {})
    if state.get("count") != STATE_N or state.get("width") != 2:
        erros.append(f"effectState count={state.get('count')} width={state.get('width')}")

    params = lay.get("params", {})
    if params.get("count") != PARAMS_N:
        erros.append(f"params count={params.get('count')} (esperado {PARAMS_N})")
    if params.get("per_slot") != PARAMS_POR_SLOT:
        erros.append(f"params per_slot={params.get('per_slot')} (esperado {PARAMS_POR_SLOT})")
    if params.get("width") != 4 or params.get("float") is not True:
        erros.append(f"params width={params.get('width')} float={params.get('float')}")

    # O span tem de cobrir exatamente os 135, sem buraco e sem sobreposicao.
    span = params.get("span")
    if not isinstance(span, list) or not span:
        erros.append("params.span ausente")
    else:
        cobertos = 0
        anterior = None
        for faixa in span:
            pg, off, cnt = faixa.get("page"), faixa.get("offset"), faixa.get("count")
            if not all(isinstance(v, int) for v in (pg, off, cnt)):
                erros.append(f"span invalido: {faixa}")
                continue
            if not (0 <= pg <= 8):
                erros.append(f"span page={pg} fora de 0..8")
            if anterior is not None and pg == anterior[0]:
                esperado = anterior[1] + 4 * anterior[2]
                if off != esperado:
                    erros.append(f"span com buraco/sobreposicao em pg{pg}: {off} != {esperado}")
            anterior = (pg, off, cnt)
            cobertos += cnt
        if cobertos != PARAMS_N:
            erros.append(f"span cobre {cobertos} params (esperado {PARAMS_N})")

    provas = a.get("provas")
    if not isinstance(provas, dict):
        erros.append("provas ausente")
    else:
        for campo, esperado in (
            ("cadeia_ok", 198), ("code_ok", 1782), ("state_ok", 1782)
        ):
            v = provas.get(campo)
            if v != esperado:
                erros.append(f"provas.{campo}={v} (esperado {esperado})")

    return erros


def main():
    a, erros = carregar()
    if a is not None:
        erros += validar(a)
    if erros:
        for e in erros:
            print(f"ERRO: {e}")
        return 1
    print(
        f"ok: taxa_params={a['taxa_params']} taxa_state={a['taxa_state']} "
        f"cadeias={a.get('cadeias_distintas')}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())

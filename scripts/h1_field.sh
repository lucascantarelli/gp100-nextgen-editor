#!/usr/bin/env bash
# h1_field.sh — runbook de campo do gate H1 (docs/H1_CHECKLIST.md).
#
# Uso:
#   ./scripts/h1_field.sh rehearsal                       # ensaio HOJE contra o MOCK
#   ./scripts/h1_field.sh field <caminho-do-cli-real> <nome-da-sessao>
#                                                         # pós-RealDevice (H1 de verdade)
#
# O que faz (Fases B e C do checklist, automatizadas):
#   B1 info → B2 list-user-irs → B3 dump-preset do pp corrente →
#   B4 dump-preset de 3 pps → e no fim chama `scripts/h1_compare.py`, que é
#   o JUIZ da Fase C: compara com as referências do mock, classifica cada
#   divergência nos 3 níveis do §4 e imprime a tabela do §5.
#
# POR QUE O JUIZ ESTÁ EM PYTHON E NÃO AQUI. Este arquivo já tinha o
# `sed`+`diff` que fazia a comparação de framing, e ele tinha dois defeitos
# que só apareceram quando alguém foi usar: (1) a Fase C do modo `field` nem
# chamava a comparação — imprimia um lembrete para o humano fazer na mão
# depois de desligar a pedaleira, que é o pior momento para improvisar; (2)
# mesmo no `rehearsal` a regra vivia em dois lugares (aqui e no Python), e
# as duas cópias divergiriam no primeiro ajuste. Agora a regra mora em UM
# lugar, e este arquivo só executa o roteiro.
#
# Regras do checklist respeitadas: LER é seguro (nenhuma escrita aqui);
# timeout = repetir 1× antes de classificar; divergência de nível 1/3 =
# PARAR e abrir o fluxo R3 (§6 do checklist). PROIBIDO nesta fase:
# set-param/save/upload-ir (mesmo dry-run — é H2) e update de firmware.

set -u  # sem -e: a classificação de divergência é manual, o roteiro segue

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOCK_CLI="$ROOT/target/release/gp100-cli.exe"
REF="$ROOT/analysis/h1_reference"
STAMP="$(date +%Y%m%d_%H%M%S)"

die() { echo "[!] $*" >&2; exit 2; }

# ---------------------------------------------------------------- helpers

# Fase C. O juiz é `scripts/h1_compare.py`: ele sabe o que é nível 1 (a ORDEM
# dos frames), o que é nível 3 (a FORMA contra o §13) e o que é nível 2
# (conteúdo de estado, esperado). Devolve 1 quando há divergência
# `bloqueia-H1`, e o runbook propaga esse código — um `field` que reprova a
# Fase C tem de reprovar o script, senão o operador lê "concluído" e segue.
fase_c() {
    # $1 = diretorio da sessao  $2 = "--ensaio" se for rehearsal
    local sessao="$1" modo="${2:-}"
    local py="python3"
    command -v python3 >/dev/null 2>&1 || py="python"
    "$py" "$ROOT/scripts/h1_compare.py" "$sessao" $modo --markdown
}

# ---------------------------------------------------------------- rehearsal

do_rehearsal() {
    [ -f "$MOCK_CLI" ] || die "binário de campo ausente: $MOCK_CLI (cargo build --release -p gp100-cli)"
    local out="$ROOT/analysis/h1_rehearsal_$STAMP"
    mkdir -p "$out"
    echo "════ ENSAIO H1 (mock) — saída em $out"
    cp "$MOCK_CLI" "$out/" 2>/dev/null || true

    # B1 — info (sem fio; estado do mock)
    "$MOCK_CLI" info > "$out/info.txt" 2>&1
    cat "$out/info.txt"
    # B2 — list-user-irs com log
    "$MOCK_CLI" list-user-irs --log "$out/b2_list.jsonl" > "$out/b2_list.txt" 2>&1
    head -5 "$out/b2_list.txt"
    # B3/B4 — dump-preset do pp corrente + 3 pps (1º, meio, fim)
    for pp in 0x0000 0x0000 0x0031 0x0062; do
        "$MOCK_CLI" dump-preset "$pp" --log "$out/b_dump_$pp.jsonl" > "$out/b_dump_$pp.txt" 2>&1
        tail -1 "$out/b_dump_$pp.txt"
    done
    # C — o juiz classifica a sessão e imprime a tabela do §5
    local rc=0
    fase_c "$out" "--ensaio" || rc=$?
    echo "════ ENSAIO concluído (juiz saiu com $rc) — logs em $out"
    echo "    Critério: níveis 1 e 3 limpos (o ensaio prova o ROTEIRO;"
    echo "    o campo prova o DEVICE). Verde aqui = kit pronto p/ campo."
    return $rc
}

# ---------------------------------------------------------------- field

do_field() {
    local real_cli="$1" tag="$2"
    [ -x "$real_cli" ] || die "CLI real não encontrado/executável: $real_cli"
    local out="$ROOT/analysis/h1_field_$tag"
    mkdir -p "$out"
    echo "════ CAMPO H1 (só leitura) — saída em $out"
    echo "    Pré-checagens do checklist §2: Suite FECHADO? USB direta? display anotado?"

    # B1 — info (1ª leitura; timeout ⇒ repetir 1× antes de classificar)
    "$real_cli" --real --i-know-what-im-doing --log "$out/b1_info.jsonl" info \
        > "$out/b1_info.txt" 2>&1 \
        || "$real_cli" --real --i-know-what-im-doing --log "$out/b1_info_r2.jsonl" info \
            > "$out/b1_info.txt" 2>&1
    cat "$out/b1_info.txt"

    # B2 — tabela dos 20 IRs
    "$real_cli" --real --i-know-what-im-doing --log "$out/b2_list.jsonl" list-user-irs \
        > "$out/b2_list.txt" 2>&1 \
        || "$real_cli" --real --i-know-what-im-doing --log "$out/b2_list_r2.jsonl" list-user-irs \
            > "$out/b2_list.txt" 2>&1
    head -23 "$out/b2_list.txt"

    # B3 — dump do pp corrente (o display manda o pp; aqui é o 1º do device)
    "$real_cli" --real --i-know-what-im-doing --log "$out/b3_dump.jsonl" dump-preset 0x0000 \
        > "$out/b3_dump.txt" 2>&1 \
        || "$real_cli" --real --i-know-what-im-doing --log "$out/b3_dump_r2.jsonl" dump-preset 0x0000 \
            > "$out/b3_dump.txt" 2>&1
    tail -2 "$out/b3_dump.txt"

    # B4 — 3 pps (1º, meio, fim do inventário REAL; ajustar após o B1/b4b)
    for pp in 0x0000 0x0031 0x0062; do
        "$real_cli" --real --i-know-what-im-doing --log "$out/b4_dump_$pp.jsonl" dump-preset "$pp" \
            > "$out/b4_dump_$pp.txt" 2>&1 \
            || "$real_cli" --real --i-know-what-im-doing --log "$out/b4_dump_${pp}_r2.jsonl" dump-preset "$pp" \
                > "$out/b4_dump_$pp.txt" 2>&1
        tail -1 "$out/b4_dump_$pp.txt"
    done

    echo "════ CAMPO concluído — logs brutos: $out/*.jsonl (append-only, -text)"
    echo
    # A FASE C É O JUIZ. Antes ela era um lembrete em texto ("compare o
    # framing de cada *.jsonl vs analysis/h1_reference/") executado na mão,
    # DEPOIS de desligar a pedaleira — e é ela que decide se o H1 passou.
    local rc=0
    fase_c "$out" || rc=$?
    if [ "$rc" -ne 0 ]; then
        echo
        echo "!!! H1 BLOQUEADO. NÃO siga o roteiro e NÃO improvise em campo:"
        echo "    §7 do checklist tem o plano de backup fixo, decidido antes de"
        echo "    ligar a pedaleira. Fluxo R3 (§6): preservar a evidência →"
        echo "    decodificar o log no PC → build_golden → validate 100% →"
        echo "    bump de baseline nos 5 lugares → retomar o código → H1 do zero."
    fi
    echo
    echo "    Preencher docs/H1_REPORT.md com a tabela do §5 acima e arquivar."
    return $rc
}

# ---------------------------------------------------------------- reference

# `analysis/h1_reference/` é um artefato DERIVADO: é a saída do MockDevice
# atual, não um golden de captura. Por isso ela tem um comando que a
# regenera — sem ele, o único jeito de consertar um drift é editar arquivo
# na mão, que é o caminho mais curto para a referência deixar de descrever
# o mock sem ninguém perceber.
#
# A regeneração NÃO é um atalho: ela roda o mesmo `rehearsal` e só copia o
# que o juiz aceitou. Se o mock divergir da §13 no shape, o juiz reprova e
# a referência não é reescrita.
do_refresh() {
    [ -f "$MOCK_CLI" ] || die "binário ausente: $MOCK_CLI (cargo build --release -p gp100-cli)"
    local forcar="${1:-}"
    local out="$ROOT/analysis/h1_refresh_$STAMP"
    mkdir -p "$out"
    "$MOCK_CLI" info > "$out/info.txt" 2>&1
    "$MOCK_CLI" list-user-irs --log "$out/b2_list.jsonl" > "$out/b2_list.txt" 2>&1
    for pp in 0x0000 0x0031 0x0062; do
        "$MOCK_CLI" dump-preset "$pp" --log "$out/b_dump_$pp.jsonl" > "$out/b_dump_$pp.txt" 2>&1
    done

    local py="python3"
    command -v python3 >/dev/null 2>&1 || py="python"
    local rc=0
    "$py" "$ROOT/scripts/h1_compare.py" "$out" --ensaio --markdown || rc=$?

    # rc 1 = divergência de FORMA (nivel 1/3). A referência nova descreveria
    # um device impossível, e nada dentro do refresh tem como julgar isso —
    # por isso ela nao e copiada NEM com --forcar. E o fluxo R3 (§6).
    if [ "$rc" -eq 1 ]; then
        echo
        echo "!!! O mock diverge da referência em FORMA (nível 1/3)."
        echo "    A referência NÃO foi reescrita e --forcar NÃO copia: aceitar"
        echo "    aqui gravaria uma referência que descreve um device impossível,"
        echo "    e o próximo H1 compararia a pedaleira contra ela."
        echo "    Caminho certo: fluxo R3 (§6) — build_golden → validate 100%"
        echo "    → bump de baseline nos 5 lugares."
        return 1
    fi

    # rc 3 = drift de CONTEUDO (nivel 2): a forma bate com o §13, os bytes
    # nao. E mudanca de SEMANTICA do protocolo (a #21 achou que a pagina
    # responde PG+1, nao PG), e nao se copia em silencio.
    if [ "$rc" -eq 3 ] && [ "$forcar" != "--forcar" ]; then
        echo
        echo "!!! Drift de CONTEÚDO (nível 2) entre o mock e a referência."
        echo "    Só o conteúdo diverge — a forma bate com o §13. Confira o diff"
        echo "    dos bytes (logs crus em $out) e, se a mudança for o mock"
        echo "    acertando a §13, prossiga com:"
        echo
        echo "      ./scripts/h1_field.sh refresh-reference --forcar"
        return 1
    fi

    cp "$out/info.txt"        "$REF/mock_info.txt"
    cp "$out/b2_list.jsonl"   "$REF/mock_list_irs.jsonl"
    cp "$out/b2_list.txt"     "$REF/mock_list_irs.txt"
    for pp in 0x0000 0x0031 0x0062; do
        cp "$out/b_dump_$pp.jsonl" "$REF/mock_dump_$pp.jsonl"
        cp "$out/b_dump_$pp.txt"   "$REF/mock_dump_$pp.txt"
    done
    "$py" "$ROOT/scripts/h1_compare.py" --self-check
    echo "════ referência regenerada a partir do mock em $REF"
    echo "    logs crus da geração: $out"
    echo "    confira o diff antes de commitar — mudança de protocolo"
    echo "    merece issue própria, não um refresh silencioso."
}

# ---------------------------------------------------------------- main

case "${1:-}" in
    rehearsal) shift; do_rehearsal "$@" ;;
    field) shift; [ $# -eq 2 ] || die "uso: h1_field.sh field <cli-real> <nome-da-sessao>"; do_field "$1" "$2" ;;
    refresh-reference) shift; do_refresh "$@" ;;
    *) cat <<'USAGE'
uso: scripts/h1_field.sh <comando>

  rehearsal                       ensaio completo contra o MOCK (hoje)
  field <cli-real> <tag>          roteiro de campo pós-RealDevice (H1)
  refresh-reference               regenera analysis/h1_reference/ do mock atual

A Fase C (comparação e classificação nos 3 níveis do §4) é feita por
scripts/h1_compare.py — chame-o direto se tiver a sessão e quiser o
veredito sem re-rodar o roteiro:

  python3 scripts/h1_compare.py analysis/h1_field_<tag> --markdown
  python3 scripts/h1_compare.py --self-check   # gate: referência x §13

Referências do mock: analysis/h1_reference/ · Checklist: docs/H1_CHECKLIST.md
USAGE
    ;;
esac

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
#   B4 dump-preset de 3 pps → comparação contra as referências do mock
#   (analysis/h1_reference/), separando FRAMING (schema/func/addr) de
#   CONTEÚDO de estado (esperado divergir — nível 2 do checklist).
#
# Regras do checklist respeitadas: LER é seguro (nenhuma escrita aqui);
# timeout = repetir 1× antes de classificar; divergência de nível 1/3 =
# PARAR e abrir o fluxo R3 (§6 do checklist). PROIBIDO nesta fase:
# set-param/save/upload-ir (mesmo dry-run — é H2) e update de firmware.

set -u  # sem -e: a classificação de divergência é manual, o roteiro segue

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REF="$ROOT/analysis/h1_reference"
MOCK_CLI="$ROOT/target/release/gp100-cli.exe"
SESSION_TAG="H1"
STAMP="$(date +%Y%m%d_%H%M%S)"

die() { echo "[!] $*" >&2; exit 2; }

# ---------------------------------------------------------------- helpers

# Framing de um log P4: linhas "func|addr" preservando a ordem de chegada
# (OUT e IN intercalados — a ordem é parte da prova, §4 nível 1).
framing_of() {
    # $1 = arquivo jsonl
    sed -E 's/.*"func":"([0-9a-f]{2})".*"addr":"([0-9a-f]{8})".*/\1|\2/' "$1"
}

# Divergência de FRAMING (nível 1): compara a sequência de func|addr dos
# dois logs, ignorando CONTEÚDO. Devolve 0 se idêntico.
framing_diff() {
    diff <(framing_of "$1") <(framing_of "$2") >/dev/null 2>&1
}

# Classifica o par (real, referência) e imprime o veredito da etapa.
compare_step() {
    # $1=nome do passo  $2=arquivo real  $3=arquivo referência (txt)  $4=log real  $5=log ref
    local name="$1" real_txt="$2" ref_txt="$3" real_log="$4" ref_log="$5"
    echo "--- [$name]"
    if [ ! -s "$real_txt" ]; then
        echo "    FALHA: saída vazia (Timeout/InvalidShape?) — repetir 1×; se persistir, R3"
        return 1
    fi
    if framing_diff "$real_log" "$ref_log"; then
        echo "    FRAMING: idêntico ao mock (nível 1 OK)"
    else
        echo "    FRAMING: DIVERGENTE (nível 1/3) — PARAR e abrir fluxo R3 (§6)"
        echo "    evidência: diff dos frames em $real_log vs $ref_log"
        return 1
    fi
    # Nível 2 (conteúdo de estado) NÃO é falha: só sinaliza diferença.
    if diff -q "$real_txt" "$ref_txt" >/dev/null 2>&1; then
        echo "    conteúdo: idêntico ao mock (estado do device = all.prst)"
    else
        echo "    conteúdo: divergente — ESPERADO por desenho (device real ≠ all.prst;"
        echo "      S2 subiu IRs, S4 gravou 'It's GP100'). Documentar no H1_REPORT §5."
    fi
    return 0
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
    # C — comparação contra a referência (deve dar FRAMING OK em tudo)
    local fails=0
    compare_step "B2 list-user-irs" "$out/b2_list.txt" "$REF/mock_list_irs.txt" \
        "$out/b2_list.jsonl" "$REF/mock_list_irs.jsonl" || fails=$((fails+1))
    for pp in 0x0000 0x0031 0x0062; do
        compare_step "dump $pp" "$out/b_dump_$pp.txt" "$REF/mock_dump_$pp.txt" \
            "$out/b_dump_$pp.jsonl" "$REF/mock_dump_$pp.jsonl" || fails=$((fails+1))
    done
    echo "════ ENSAIO concluído ($fails divergências de framing) — logs em $out"
    echo "    Critério: FRAMING idêntico em todas as etapas (o ensaio prova o ROTEIRO;"
    echo "    o campo prova o DEVICE). Se tudo verde aqui, o kit está pronto p/ campo."
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
    echo "    Comparação (Fase C): framing de cada *.jsonl vs analysis/h1_reference/"
    echo "    Divergência de nível 1/3 ⇒ PARAR e seguir o fluxo R3 (§6 do checklist):"
    echo "    logar → captura nova (se Suite necessário) → build_golden → validate 100%"
    echo "    → bump de baseline nos 5 lugares → retomar."
    echo "    Preencher docs/H1_REPORT.md com a tabela do §5."
}

# ---------------------------------------------------------------- main

case "${1:-}" in
    rehearsal) shift; do_rehearsal "$@" ;;
    field) shift; [ $# -eq 2 ] || die "uso: h1_field.sh field <cli-real> <nome-da-sessao>"; do_field "$1" "$2" ;;
    *) cat <<'USAGE'
uso: scripts/h1_field.sh <comando>

  rehearsal                       ensaio completo contra o MOCK (hoje)
  field <cli-real> <tag>          roteiro de campo pós-RealDevice (H1)

Referências do mock: analysis/h1_reference/ · Checklist: docs/H1_CHECKLIST.md
USAGE
    ;;
esac

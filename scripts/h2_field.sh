#!/usr/bin/env bash
# h2_field.sh — runbook de campo do gate H2 (docs/H2_CHECKLIST.md), issue #22.
#
# Uso:
#   ./scripts/h2_field.sh rehearsal                        # ensaio HOJE contra o MOCK
#   ./scripts/h2_field.sh refresh-reference                # regenera analysis/h2_reference/
#   ./scripts/h2_field.sh field <caminho-do-cli> <nome>    # o H2 de verdade (escrita)
#
# O que faz (Fases B e C do checklist, automatizadas):
#   F1 set-param → F2 save → F3 upload-ir, UM POR VEZ, e no fim chama
#   `scripts/h2_compare.py`, que é o JUIZ da Fase C: confere os invariantes
#   do §13 nos 3 logs e imprime a tabela do §6.
#
# POR QUE UM RUNBOOK PARA O H2 E NÃO SÓ PARA O H1. O H1 é leitura: rodar o
# roteiro errado custa uma reconexão. O H2 é a ÚNICA operação do projeto que
# muda o estado de um aparelho do usuário, e o erro de roteiro aqui grava um
# preset errado ou occupy um slot de IR — o preset do dono não volta sozinho.
# Por isso este script tem três coisas que o `h1_field.sh` não tem:
#
#   1. `PAUSA` entre os fluxos, com o que o operador tem que LER no display
#      antes de seguir. O checklist §2.1 diz "um por vez"; um script que
#      dispara os 3 seguidos estaria contradizendo o próprio checklist.
#   2. A escrita NUNCA vai por diante depois de uma reprovação. O `set -e`
#      está aqui de propósito (o `h1_field.sh` usa `set -u` e segue, porque
#      lá a resposta a divergência é outra).
#   3. O `--log` é OBRIGATÓRIO em todo fluxo. Sem log não há prova, e sem
#      prova o veredito vira "pareceu que" — que é o que o §6 proíbe.
#
# O que este script NÃO faz, por desenho: ele não julga o display. O fio
# não sabe se o knob girou nem se o nome persistiu (D3/D4: os dois são
# fire-and-forget, sem read-back). O juiz imprime o valor que o display
# DEVERIA mostrar; a comparação é do operador.

set -eu  # -e de propósito: no H2 uma reprovação para o roteiro

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MOCK_CLI="$ROOT/target/release/gp100-cli.exe"
REF="$ROOT/analysis/h2_reference"
STAMP="$(date +%Y%m%d_%H%M%S)"

die() { echo "[!] $*" >&2; exit 2; }

py() {
    local p="python3"
    command -v python3 >/dev/null 2>&1 || p="python"
    "$p" "$@"
}

# Binário de campo do H2: o H1 é `--features real-device`, e o H2 é o mesmo
# binário MAIS `write-verified` (ADR-5). Sem essa feature a escrita é
# IMPOSSÍVEL — não há flag que destrave — e o `gp100-cli` recusa com o
# próprio texto, que é a prova de que a trava funciona.
binario_campo() {
    [ -f "$MOCK_CLI" ] || die "binário ausente: $MOCK_CLI
    O H2 exige o build de escrita (ADR-5):
      cargo build --release -p gp100-cli --features real-device,write-verified"
}

# O blob de ensaio: 75B = 5 chunks de 15B exatos (§13.7 rev.2 — a FSM
# REJEITA o resto, não padroniza). Os bytes são um gradiente, não zeros: um
# blob de zeros passa no tamanho e esconde erro de ordem de nibble.
blob_de_ensaio() {
    py -c "
import pathlib, sys
n = int(sys.argv[1])
pathlib.Path(sys.argv[2]).write_bytes(bytes((i * 7 + 3) & 0xff for i in range(n)))
" "${1:-75}" "$2"
}

# ------------------------------------------------------------------ pausa

# A pausa entre os fluxos. Sem ela o script dispara os 3 e o operador perde o
# "um por vez" do §2.1 — e sem o display lido entre eles, o §2.5 ("o display
# é a verdade") não tem em que se apoiar.
pausa() {
    echo
    echo "────────────────────────────────────────────────────────────"
    echo "  $1"
    echo "  $2"
    # A pausa é a REGRA 2.1 do checklist ("um por vez"), mas ela não pode
    # travar o script quando não há ninguém para apertar Enter — que é o caso
    # do ensaio em CI, de um pipe, ou de um operador com pressa. Sem TTY a
    # pausa vira aviso, e a diferença fica explícita na saída em vez de
    # silenciosa: quem roda sem TTY não está no meio de uma sessão de
    # campo, e o display não existe para ser lido.
    if [ ! -t 0 ]; then
        echo "  (sem TTY: pausa pulada — isto NÃO é uma sessão de campo)"
        return 0
    fi
    echo "  Enter para seguir (Ctrl-C para PARAR aqui)..."
    read -r _ || true
}

# ───────────────────────────────────────────────────────────────── rehearsal

do_rehearsal() {
    binario_campo
    local out="$ROOT/analysis/h2_rehearsal_$STAMP"
    mkdir -p "$out"
    echo "════ ENSAIO H2 (mock) — saída em $out"
    echo "O ensaio prova o ROTEIRO e a tabela de invariantes do juiz."
    echo "Ele NAO diz nada sobre o seu GP-100: nenhum byte sai daqui."
    echo

    # F1 — set-param (§13.11): 1 frame OUT, ZERO IN (D4).
    "$MOCK_CLI" --log "$out/f1_setparam.jsonl" set-param 1 0x0700006e 0 99.5 \
        > "$out/f1_setparam.txt" 2>&1
    tail -2 "$out/f1_setparam.txt"
    pausa "F1 rodado contra o MOCK." "Em campo, o display tem que mostrar 99.5 no knob."

    # F2 — save (§13.12): 9 frames OUT, ZERO IN (D3).
    "$MOCK_CLI" --log "$out/f2_save.jsonl" save 0x0000 4 "H2 TESTE" \
        > "$out/f2_save.txt" 2>&1
    tail -3 "$out/f2_save.txt"
    pausa "F2 rodado contra o MOCK." "Em campo, o preset tem que mostrar 'H2 TESTE' e manter depois de sair e voltar."

    # F3 — upload-ir (§13.7): BEGIN + N chunks, cada um com o ACK dele.
    blob_de_ensaio 75 "$out/ir_blob.bin"
    "$MOCK_CLI" --log "$out/f3_upload.jsonl" upload-ir 0 "$out/ir_blob.bin" \
        > "$out/f3_upload.txt" 2>&1
    tail -2 "$out/f3_upload.txt"
    pausa "F3 rodado contra o MOCK." "Em campo, o slot 0 tem que mostrar o nome do IR."

    echo
    echo "════ FASE C (juiz: scripts/h2_compare.py)"
    local codigo=0
    py "$ROOT/scripts/h2_compare.py" "$out" || codigo=$?
    echo
    if [ "$codigo" -eq 0 ]; then
        echo "════ ENSAIO concluído (juiz saiu com 0)"
        echo "    Critério: os 3 fluxos obedeceram o §13 contra o MOCK."
        echo "    O campo prova o DEVICE. Verde aqui = kit pronto p/ campo."
        echo "    Para ancorar a tabela de formas: ./scripts/h2_field.sh refresh-reference"
    else
        echo "════ ENSAIO REPROVADO (juiz saiu com $codigo)"
        echo "    Não siga para o campo: o problema está no software, não na"
        echo "    pedaleira. Veja os achados acima."
        exit 1
    fi
}

# ─────────────────────────────────────────────────────────── refresh-reference

# Gera a referencia que ancora a `FORMAS_13` do juiz. É ela que impede a
# transcrição do §13 de envelhecer em silêncio — o self-check compara a
# tabela contra estes logs (ver `self_check` no h2_compare.py).
do_refresh_reference() {
    binario_campo
    [ -d "$REF" ] || mkdir -p "$REF"
    echo "════ REFERÊNCIA — regenerando $REF a partir do MOCK"
    "$MOCK_CLI" --log "$REF/f1_setparam.jsonl" set-param 1 0x0700006e 0 99.5 >/dev/null 2>&1
    "$MOCK_CLI" --log "$REF/f2_save.jsonl" save 0x0000 4 "H2 TESTE" >/dev/null 2>&1
    blob_de_ensaio 75 "$REF/ir_blob.bin"
    "$MOCK_CLI" --log "$REF/f3_upload.jsonl" upload-ir 0 "$REF/ir_blob.bin" >/dev/null 2>&1
    echo "    f1/f2/f3 regenerados. Confira o diff antes de versionar:"
    echo "      git diff --stat $REF"
    echo "    Um diff aqui que NAO seja o blob pode ser um mock que regrediu."
    py "$ROOT/scripts/h2_compare.py" --self-check
}

# ──────────────────────────────────────────────────────────────────── field

# O H2 de verdade. `$1` = caminho do binário compilado com
# `--features real-device,write-verified`; `$2` = nome da sessão.
do_field() {
    local cli="${1:-}" sessao="${2:-}"
    [ -n "$cli" ] || die "uso: $0 field <caminho-do-binario-de-escrita> <nome-da-sessao>"
    [ -x "$cli" ] || die "binário não encontrado ou não executável: $cli"
    local out="$ROOT/analysis/h2/${sessao:-$STAMP}"
    mkdir -p "$out"

    cat <<AVISO
════════════════════════════════════════════════════════════════════════
  H2 — ESCRITA REAL NO APPARELHO DO DONO.

  Esta é a única operação do projeto que muda o estado de um
  aparelho de verdade. Antes de continuar:

    1. o GP-100 está na USB direta, firmware V2.1, Suite FECHADO;
    2. você anotou quais slots de IR estão OCUPADOS e quais
       presets são seus (upload em slot ocupado não tem volta);
    3. o preset que vai receber o set-param e o save é
       DESCARTÁVEL (regra 3.2 — o save grava o estado ao vivo);
    4. o H1 passou e está arquivado (escrever num device cuja
       leitura ainda diverge é o pior cenário possível).

  Responda 'ok' para começar. Qualquer outra coisa aborta.
════════════════════════════════════════════════════════════════════════
AVISO
    local resposta=""
    if [ -t 0 ]; then
        read -r resposta || true
        [ "$resposta" = "ok" ] || die "abortado pelo operador (nenhum byte saiu)"
    else
        # Sem TTY não há operador para confirmar, e ESTE é o script que
        # escreve no aparelho de alguém. Recusar é o único desfecho que
        # não depende deeu adivinhar a intenção.
        die "sem TTY: o H2 exige o 'ok' digitado por uma pessoa.
    Rodar o gate de escrita por pipe, sem alguém para olhar o display
    entre os fluxos, contradiz a regra 2.1 do H2_CHECKLIST."
    fi

    echo "Sessão em $out"

    # F1 — set-param. Regra 3.2: preset descartável, knob com valor visível.
    "$cli" --real --i-know-what-im-doing --log "$out/f1_setparam.jsonl" \
        set-param 1 0x0700006e 0 99.5 > "$out/f1_setparam.txt" 2>&1
    cat "$out/f1_setparam.txt"
    pausa "F1 ENVIADO ao aparelho." "Olhe o DISPLAY: o valor do knob mudou para 99.5?
  Anote o veredito: mudou · não mudou · device reagiu estranho.
  (O fio NÃO tem read-back deste fluxo — D4. O display é a prova.)"

    # F2 — save. 9 frames, zero resposta.
    "$cli" --real --i-know-what-im-doing --log "$out/f2_save.jsonl" \
        save 0x0000 4 "H2 TESTE" > "$out/f2_save.txt" 2>&1
    cat "$out/f2_save.txt"
    pausa "F2 ENVIADO ao aparelho." "Olhe o DISPLAY: o nome virou 'H2 TESTE'?
  Depois SAIA do preset e VOLTE — é isso que prova persistência, e não só cache.
  Anote o veredito: persistiu · mudou mas não persiste · não mudou.
  (O fio NÃO tem read-back deste fluxo — D3. Um burst tardio NÃO é confirmação.)"

    # F3 — upload-ir. Regra 5: slot VAZIO, confirmado antes com list-user-irs.
    echo
    echo "── ANTES do F3: confirme que o slot está VAZIO ──"
    "$cli" --real --i-know-what-im-doing list-user-irs 2>&1 | head -25
    pausa "Inventário acima." "Escolha um slot VAZIO e passe o número no lugar do 0 abaixo.
  Regra dura do §5: NÃO subir em slot ocupado — o conteúdo anterior não
  se recupera. Se não houver slot vazio, PARE aqui e escolha outra sessão."

    blob_de_ensaio 75 "$out/ir_blob.bin"
    "$cli" --real --i-know-what-im-doing --log "$out/f3_upload.jsonl" \
        upload-ir 0 "$out/ir_blob.bin" > "$out/f3_upload.txt" 2>&1
    cat "$out/f3_upload.txt"
    pausa "F3 ENVIADO ao aparelho." "Olhe o DISPLAY: o slot mostra o nome do IR?
  E rode de novo o list-user-irs: o slot não pode voltar vazio nem 0xFF.
  Anote o veredito: slot ocupado com o nome · slot vazio · device travou."

    # Read-back de software — a ÚNICA verificação programática dos 3 fluxos.
    "$cli" --real --i-know-what-im-doing --log "$out/readback.jsonl" \
        list-user-irs > "$out/readback.txt" 2>&1 || true
    tail -25 "$out/readback.txt"

    echo
    echo "════ FASE C (juiz: scripts/h2_compare.py)"
    local codigo=0
    py "$ROOT/scripts/h2_compare.py" "$out" || codigo=$?
    echo
    echo "════ Tabela do §6 (cole no docs/H2_REPORT.md com o veredito de cada um):"
    py "$ROOT/scripts/h2_compare.py" "$out" --markdown
    echo
    if [ "$codigo" -eq 0 ]; then
        echo "O fio obedeceu o §13 nos 3 fluxos."
        echo "AINDA NÃO É O H2 VERDE: falta o veredito do display de cada fluxo"
        echo "no relatório. Um veredito por escrito, não 'pareceu que' (§2.5)."
    else
        echo "PARAR. Nao repetir o fluxo às cegas, nao trocar de slot (§6)."
        echo "Preserve o log e a saída ANTES de desligar, e classifique:"
        echo "  dispositivo respondeu diferente do §13 -> PROTOCOLO (fluxo R3);"
        echo "  respondeu o §13 mas o efeito não foi o esperado -> COMPORTAMENTO,"
        echo "  e vira issue nova. Não se conserta mexendo no codec (R1)."
    fi
    exit "$codigo"
}

# ────────────────────────────────────────────────────────────────────── main

case "${1:-}" in
    rehearsal) do_rehearsal ;;
    refresh-reference) do_refresh_reference ;;
    field) shift; do_field "${1:-}" "${2:-}" ;;
    *)
        sed -n '3,12p' "$0" | sed 's/^# \{0,1\}//'
        exit 2
        ;;
esac

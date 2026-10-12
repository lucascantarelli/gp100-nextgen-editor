# Extração bruta da tabela `algorithmParaNameConstData` (work opcional)

Objetivo: confirmar in-place os nomes/contagens de parâmetros direto da firmware V2.1,
sem depender do `algorithm.xml` do app oficial. O `parameters.json` atual já é
funcional e validado; este documento registra o caminho para auditoria futura.

## Estado atual da investigação
- Strings dos nomes estão na região `0x279xxx–0x281xxx` do binário (arquivo, sem base).
- **Não há ponteiros absolutos** para essas strings no binário: o binário é uma imagem
  de flash XIP carregada em um endereço base desconhecido, e o código Cortex-M7
  (Thumb-2) tipicamente monta endereços com literal pools que contêm
  `base + offset` pré-somados OU pares MOVW/MOVT.
- Sonar de base (varredura K×base procurando ponteiros para 5 strings conhecidas)
  não encontrou base em {0x00000000, 0x08000000, 0x20000000, 0x20200000, 0x30000000,
  0x60000000, 0x60800000, 0x68000000, 0x70000000} com K em 0..0x2000000 (4 KB step).
  Possíveis causas: endereços construídos por MOVW/MOVT (nunca aparecem como u32
  completos), ou região de nomes acessada via offset relativo a um ponteiro de
  estrutura (PC-relative `ADR`/`ADD`).

## Caminho Ghidra recomendado
1. Importar `GP-100 Firmware V2.1.bin` como ARM Cortex-M7 little-endian (Thumb-2),
   base sugerida `0x60000000` (FlexSPI XIP do i.MX RT).
2. Primeiro, carregar a entrada a partir do vetor de reset: procurar o vetor
   (padrão Cortex-M: SP em 0x2000xxxx seguido de reset odd) na imagem.
3. Rodar `Auto-Analyze` + `Aggressive Instruction Finder`.
4. Scripts:
   - `find-crc-table` genérico: localizar a tabela CRC-8/0x07 ou CRC-32 para ancorar
     funções de verificação de pacotes.
   - Buscar referências à string `algorType < MAX_ALGOR_NUM` (assert em
     `algorithmParaNameConstData.c`); a função que contém esse assert é o getter
     `GetAlgorParaName(algorType, algor)`; o acesso à tabela de nomes fica em
     literal pool adjacente (LDR rX, =const) — o valor é `tabela - base` ou
     `base + tabela` e revela a organização da tabela.
   - A tabela esperada é `char* names[MAX_ALGOR_NUM][max_paras]` ou uma estrutura
     `{name, para_count, para_names[]}` por algoritmo; as contagens por módulo
     conferem com `algorithm.xml` (PRE 16, DST 19, AMP 51, CAB 60, EQ 3, MOD 12,
     DLY 12, RVB 10, NR 2 — nota: `algorithm.xml` agrupa por módulo).
5. Alternativa sem base: varrer todos os pares MOVW/MOVT que montam endereços na
   janela dos nomes (0x279xxx–0x281xxx + base) e construir histograma de offsets
   referenciados — aponta os limites das estruturas de tabela.

## Por que é opcional
- `algorithm.xml` (185 algoritmos) cobre 100% dos 116 códigos observados em 909
  slots de patches reais, com zero conflitos de range nos valores de params_0..14.
- 150 nomes de algoritmos conferem literalmente com strings da firmware; os 12
  "não encontrados" são variantes de escrita (ex.: firmware tem `SnapTone1..5`
  junto, `COMP  ` com espaços, `Hall` dentro de listas maiores).
- A firmware é a mesma referência do app oficial; divergências só apareceriam em
  firmwares futuras — nesse caso, repetir o merge com o novo `algorithm.xml`.

#!/usr/bin/env python3
"""wirelog_compare.py — o núcleo comum dos juízes de campo (H1/H2).

POR QUE ESTE MÓDULO EXISTE (auditoria 07/10, F-07, issue #141). `h1_compare.py`
(584 linhas) e `h2_compare.py` (827 linha) cresceram cada um com a SUA cópia do
mesmo par de primitivas: o `Frame` do schema P4 e o `ler_log` que o parseia. As
duas cópias nasceram byte-idênticas — e é exatamente isso que a duplicação é:
uma correção no parser de um lado (ex.: tolerar `-text` da `.gitattributes`,
aceitar campo novo no schema) fica de fora do outro, e os dois juízes passam a
concordar em silêncio DESACORDO. O `checar_formas`/`tabela`/self-check são
SEMELHANTES mas não idênticos (H1 compara contra FORMAS_13; H2 confere
INVARIANTES), então ficam nos juízes; o que é literalmente igual desce para cá.

O QUE É O SCHEMA P4 (o contrato que ambos os juízes consomem). Log de fio do
`--log` (`wire_log.rs` do core), 1 frame JSON por linha:

    {"s": <seq>, "dir": "in"|"out", "func": "12", "addr": "13010003",
     "data": "fa01c8..."}

`dir`/`func`/`addr`/`data` são OBRIGATÓRIOS; linha vazia é pulada; linha
não-JSON ou com campo faltando é ERRO de uso (não divergência — o juiz não
confronta um log quebrado com a spec, ele reclama do log).

Consumidores: `h1_compare.py`, `h2_compare.py` e os testes
`analysis/tests/test_h1_compare.py`/`test_h2_compare.py` (importam dos juízes,
que reexportam `Frame`/`ler_log` daqui).
"""

from __future__ import annotations

import io
import json
from dataclasses import dataclass
from pathlib import Path

__all__ = ["Frame", "ler_log", "rotulo_de_endpoint", "encurta_hex"]


@dataclass(frozen=True)
class Frame:
    """UM frame de fio no schema P4 (§13.1)."""

    dir: str
    func: str
    addr: str
    data: str

    @property
    def endpoint(self) -> tuple[str, str, str]:
        """A chave de classificação: (direção, FUNC, endereço 8 hex)."""
        return (self.dir, self.func, self.addr)

    @property
    def tamanho(self) -> int:
        """Bytes de payload (o `data` é hex — 2 chars por byte)."""
        return len(self.data) // 2

    @property
    def bytes(self) -> bytes:
        """Payload cru (o h2 decodifica f32/ordens de save a partir daqui)."""
        return bytes.fromhex(self.data)


def ler_log(caminho: Path) -> list[Frame]:
    """Le um log P4 (`{"s","dir","func","addr","data"}`, 1 frame por linha).

    ValueError com nome do arquivo em linha quebrada — o juiz precisa saber
    QUAL parte do log está ruim antes de confrontar qualquer referência.
    """
    frames: list[Frame] = []
    with io.open(caminho, encoding="utf-8") as fh:
        for linha in fh:
            linha = linha.strip()
            if not linha:
                continue
            try:
                bruto = json.loads(linha)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{caminho.name}: linha nao-JSON ({exc})") from exc
            faltando = {"dir", "func", "addr", "data"} - bruto.keys()
            if faltando:
                raise ValueError(
                    f"{caminho.name}: linha sem {sorted(faltando)} (schema P4)"
                )
            frames.append(
                Frame(bruto["dir"], bruto["func"], bruto["addr"], bruto["data"])
            )
    return frames


def rotulo_de_endpoint(endpoint: tuple[str, str, str]) -> str:
    """`"12/13010003"` — o rótulo curto da tabela do relatório."""
    return f"{endpoint[1]}/{endpoint[2]}"


def encurta_hex(hexa: str) -> str:
    """Encurta hex longo para caber na coluna da tabela do §5/§6."""
    return hexa if len(hexa) <= 16 else hexa[:16] + "..."
